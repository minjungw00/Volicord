use super::*;

impl CoreService {
    /// Executes `volicord.prepare_write` through the shared Core mutation pipeline.
    pub fn prepare_write(
        &self,
        request: PrepareWriteRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        if let Some(envelope_task_id) = request.envelope.task_id.as_ref() {
            if request
                .task_id
                .as_ref()
                .is_some_and(|task_id| task_id != envelope_task_id)
            {
                return validation_rejected(
                    request.envelope.dry_run,
                    None,
                    "task_id",
                    "envelope.task_id must match PrepareWriteRequest.task_id",
                );
            }
        }
        let policy = prepare_write_policy(&request);
        let prepared = match prepare_or_response(
            self,
            MethodName::PrepareWrite,
            request.envelope.clone(),
            request_json,
            invocation,
            policy,
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let plan = match plan_prepare_write(
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
                    dry_run_summary: plan.dry_run_summary,
                },
            );
        }

        self.execute_prepared_request(
            prepared,
            OwnerPipelineBranch::CommitMutation {
                result_fields: plan.result_fields,
                event_kind: plan.event_kind,
                event_payload: plan.event_payload,
                task_id: Some(plan.task_id),
                change_unit_id: plan.change_unit_id,
                storage_mutations: plan.storage_mutations,
            },
        )
    }
}

fn prepare_write_policy(request: &PrepareWriteRequest) -> MethodPolicy {
    let task = request
        .task_id
        .clone()
        .or_else(|| request.envelope.task_id.as_ref().cloned())
        .map(TaskRequirement::Exact)
        .unwrap_or(TaskRequirement::Required);

    if request.envelope.dry_run {
        MethodPolicy::exact(
            request.operation_category(),
            task,
            ReplayPolicy::None,
            FreshnessPolicy::IfPresent,
            MethodEffectPolicy::DryRunPreview,
        )
    } else {
        MethodPolicy::exact(
            request.operation_category(),
            task,
            ReplayPolicy::Committed,
            FreshnessPolicy::IfPresent,
            MethodEffectPolicy::CoreMutation,
        )
    }
}

fn plan_prepare_write(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: PrepareWriteRequest,
    verified_invocation: &VerifiedInvocationContext,
) -> Result<PrepareWritePlan, PlanError> {
    if request.intended_operation.trim().is_empty() {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "intended_operation",
            "intended_operation must not be empty",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    let normalized_operation = request.intended_operation.trim().to_owned();
    let normalized_sensitive_categories = normalized_string_set(&request.sensitive_categories);

    let normalized_paths = match normalize_product_paths(
        &store.project_record().repo_root,
        &request.intended_paths,
    ) {
        Ok(paths) => paths,
        Err(ProductPathError::Invalid) => {
            validation_plan_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "intended_paths",
                "intended_paths must be relative Product Repository paths that stay inside the repository",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
        Err(ProductPathError::LocalAccess) => {
            let response = rejected_pipeline_response(
                request.envelope.dry_run,
                Some(project_state.state_version),
                vec![tool_error(
                    ErrorCode::InvocationContextMismatch,
                    "intended_paths resolve outside the Product Repository",
                    false,
                    None,
                )],
            )
            .map_err(PlanError::Core)?;
            return Err(PlanError::Response(Box::new(response)));
        }
    };

    let planned_state_version = project_state.state_version + 1;
    let plan_now = utc_timestamp(service.now());
    let (task_id, task, mut reasons) = resolve_prepare_write_task(store, project_state, &request)?;
    let current_change_unit = store.current_change_unit(&task_id).map_err(|error| {
        PlanError::Response(Box::new(store_error_response(
            &request.envelope,
            project_state,
            error,
        )))
    })?;
    let change_unit = resolve_prepare_write_change_unit(
        &request,
        &task_id,
        current_change_unit.as_ref(),
        &mut reasons,
    );

    if request.product_file_write_intended == normalized_paths.is_empty() {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::WriteCompatibility,
            "product_write_flag_mismatch",
            "product_file_write_intended must match the intended Product Repository paths.",
            Vec::new(),
        ));
    }

    if let Some(change_unit) = change_unit {
        if !baseline_matches(change_unit, &task, &request.baseline_ref)? {
            reasons.push(write_decision_reason(
                WriteDecisionCategory::Baseline,
                "baseline_mismatch",
                "baseline_ref does not match the current write-compatibility basis.",
                vec![change_unit_ref(
                    &request.envelope.project_id,
                    &task_id,
                    change_unit,
                    project_state.state_version,
                )],
            ));
        }

        if !paths_match_current_change_unit(
            &store.project_record().repo_root,
            &normalized_paths,
            change_unit,
        )? {
            reasons.push(write_decision_reason(
                WriteDecisionCategory::Scope,
                "path_out_of_scope",
                "One or more intended paths are outside the current Change Unit path scope.",
                vec![change_unit_ref(
                    &request.envelope.project_id,
                    &task_id,
                    change_unit,
                    project_state.state_version,
                )],
            ));
        }

        if let Some(contract) = change_unit_effect_contract(change_unit)? {
            let contract_violations = product_write_violations(
                &store.project_record().repo_root,
                &contract,
                request.product_file_write_intended,
                &normalized_paths,
            )
            .map_err(|_| {
                CorePipelineError::Store(StoreError::corrupt_owner_state_json(
                    "change_units",
                    change_unit.change_unit_id.clone(),
                    "effect_contract_json",
                ))
            })?;
            for violation in contract_violations {
                reasons.push(effect_contract_reason(
                    violation,
                    change_unit_ref(
                        &request.envelope.project_id,
                        &task_id,
                        change_unit,
                        project_state.state_version,
                    ),
                ));
            }
        }
    }

    let current_change_unit_id =
        change_unit.map(|record| ChangeUnitId::new(record.change_unit_id.clone()));
    let task_ref = state_ref(
        StateRecordKind::Task,
        task_id.as_str(),
        &request.envelope.project_id,
        Some(&task_id),
        Some(project_state.state_version),
    );
    let mut operation_refs = vec![task_ref.clone()];
    if let Some(change_unit) = change_unit {
        operation_refs.push(change_unit_ref(
            &request.envelope.project_id,
            &task_id,
            change_unit,
            project_state.state_version,
        ));
    }
    let sensitive_requirement = if normalized_sensitive_categories.is_empty() {
        None
    } else {
        current_change_unit_id
            .as_ref()
            .map(|change_unit_id| SensitiveApprovalRequirement {
                task_id: &task_id,
                change_unit_id,
                scope_revision: task.scope_revision,
                operation: &normalized_operation,
                normalized_paths: &normalized_paths,
                sensitive_categories: &normalized_sensitive_categories,
                baseline_ref: Some(&request.baseline_ref),
                required_for: JudgmentRequiredFor::PrepareWrite,
                now: &plan_now,
                repo_root: &store.project_record().repo_root,
            })
    };
    let pending_authorities =
        pending_judgment_authorities_for_plan(store, project_state, &request.envelope, &task_id)?;
    let operation_context = JudgmentOperationContext {
        operation: JudgmentOperation::PrepareWrite,
        task_id: &task_id,
        change_unit_id: current_change_unit_id.as_ref(),
        scope_revision: task.scope_revision,
        close_basis: None,
        operation_refs: &operation_refs,
        sensitive_approval: sensitive_requirement.as_ref(),
    };
    let pending_user_judgment_refs = pending_authorities
        .iter()
        .filter(|authority| judgment_blocks_operation(authority, &operation_context))
        .map(|authority| {
            state_ref(
                StateRecordKind::UserJudgment,
                &authority.judgment_id,
                &request.envelope.project_id,
                Some(&task_id),
                Some(project_state.state_version),
            )
        })
        .collect::<Vec<_>>();
    if !pending_user_judgment_refs.is_empty() {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::UserJudgment,
            "user_judgment_unresolved",
            "A user-owned judgment required before write preparation remains unresolved.",
            pending_user_judgment_refs.clone(),
        ));
    }

    let mut active_user_judgment_refs = Vec::new();
    if !normalized_sensitive_categories.is_empty() {
        let matching_sensitive_approval = matching_sensitive_approval(SensitiveApprovalSearch {
            store,
            project_state,
            request: &request,
            task_id: &task_id,
            task: &task,
            change_unit,
            intended_operation: &normalized_operation,
            normalized_paths: &normalized_paths,
            sensitive_categories: &normalized_sensitive_categories,
            now: &plan_now,
        })?;
        if let Some(record) = matching_sensitive_approval {
            active_user_judgment_refs.push(state_ref(
                StateRecordKind::UserJudgment,
                &record.judgment_id,
                &request.envelope.project_id,
                Some(&task_id),
                Some(project_state.state_version),
            ));
        } else {
            reasons.push(write_decision_reason(
                WriteDecisionCategory::SensitiveApproval,
                "sensitive_approval_missing",
                "A matching sensitive-action approval is required before write ticket issuance.",
                Vec::new(),
            ));
        }
    }

    let guarantee_display = Some(guarantee_display_for_invocation(
        store,
        verified_invocation,
        planned_state_version,
    )?);
    let branch_change_unit_id =
        change_unit.map(|record| ChangeUnitId::new(record.change_unit_id.clone()));
    let scope_change_unit_id = branch_change_unit_id.clone().unwrap_or_else(|| {
        request
            .change_unit_id
            .clone()
            .unwrap_or_else(|| ChangeUnitId::new("missing_current_change_unit"))
    });
    let decision = prepare_write_decision(&reasons);
    let allowed = reasons.is_empty();
    let create_write_ticket = allowed && !request.envelope.dry_run;
    let write_ticket_id = if create_write_ticket {
        Some(allocate_write_ticket_id(service, store).map_err(PlanError::Core)?)
    } else {
        None
    };
    let attempt_scope = WriteTicketAttemptScope {
        task_id: task_id.clone(),
        change_unit_id: scope_change_unit_id.clone(),
        intended_operation: normalized_operation,
        intended_paths: normalized_paths.clone(),
        product_file_write_intended: request.product_file_write_intended,
        sensitive_categories: normalized_sensitive_categories,
        baseline_ref: Some(request.baseline_ref.clone()),
    };
    let attempt_scope_json = serde_json::to_string(&attempt_scope)?;
    let created_at = plan_now.to_string();
    let expires_at_timestamp = utc_timestamp(write_ticket_expires_at(*plan_now.as_datetime()));
    let expires_at = expires_at_timestamp.to_string();
    let write_ticket_id = write_ticket_id
        .as_ref()
        .map(|write_ticket_id| WriteTicketId::new(write_ticket_id.as_str().to_owned()));
    let write_ticket_ref = write_ticket_id.as_ref().map(|write_ticket_id| {
        state_ref(
            StateRecordKind::WriteTicket,
            write_ticket_id.as_str(),
            &request.envelope.project_id,
            Some(&task_id),
            Some(planned_state_version),
        )
    });
    let denied_path_patterns = denied_write_ticket_paths(&reasons, &normalized_paths);
    let allowed_path_patterns = normalized_paths
        .iter()
        .filter(|path| !denied_path_patterns.iter().any(|denied| denied == *path))
        .cloned()
        .collect::<Vec<_>>();
    let synthetic_write_ticket =
        write_ticket_id
            .as_ref()
            .map(|write_ticket_id| WriteTicketRecord {
                project_id: request.envelope.project_id.as_str().to_owned(),
                write_ticket_id: write_ticket_id.as_str().to_owned(),
                task_id: task_id.as_str().to_owned(),
                change_unit_id: Some(scope_change_unit_id.as_str().to_owned()),
                basis_state_version: planned_state_version,
                status: "active".to_owned(),
                attempt_scope_json: attempt_scope_json.clone(),
                expires_at: expires_at.clone(),
                created_at: created_at.clone(),
                consumed_by_run_id: None,
                consumed_at: None,
            });

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
    let evidence_summary = projected_evidence_summary(
        store,
        &request.envelope.project_id,
        planned_state_version,
        &task,
    )?;
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
        close_context_from_projection(
            task.clone(),
            change_unit.cloned(),
            projected_close_basis(store, &task_id)?,
            pending_user_judgment_refs.clone(),
            blocker_refs.clone(),
            evidence_summary.clone(),
        ),
        *plan_now.as_datetime(),
    )?;
    let mut close_state = close_plan.close_state;
    let mut close_blockers = close_plan.blockers;
    if create_write_ticket {
        if let Some(write_ticket_ref) = write_ticket_ref.as_ref() {
            let planned_task_ref = state_ref(
                StateRecordKind::Task,
                task_id.as_str(),
                &request.envelope.project_id,
                Some(&task_id),
                Some(planned_state_version),
            );
            close_blockers.insert(
                0,
                close_task::open_write_ticket_close_blocker(
                    planned_task_ref,
                    write_ticket_ref.clone(),
                ),
            );
            close_state = CloseState::Blocked;
        }
    }
    let control_surface = close_plan
        .guard_health
        .as_ref()
        .map(|guard_health| guard_health.control_surface.clone());
    let write_ticket = match (write_ticket_id.as_ref(), write_ticket_ref.as_ref()) {
        (Some(write_ticket_id), Some(write_ticket_ref)) => Some(WriteTicket {
            write_ticket_id: write_ticket_id.clone(),
            write_ticket_ref: write_ticket_ref.clone(),
            state: WriteTicketState::Open,
            scope: WriteTicketScope {
                task_id: task_id.clone(),
                change_unit_id: scope_change_unit_id.clone(),
                intended_operation: attempt_scope.intended_operation.clone(),
                product_file_write_intended: attempt_scope.product_file_write_intended,
                sensitive_categories: attempt_scope.sensitive_categories.clone(),
                baseline_ref: attempt_scope.baseline_ref.clone(),
            },
            path_patterns: WriteTicketPathPatterns {
                allowed: allowed_path_patterns.clone(),
                denied: denied_path_patterns.clone(),
            },
            observed_paths: Vec::new(),
            basis_state_version: planned_state_version,
            expires_at: Some(expires_at_timestamp.clone()),
            control_surface: control_surface.clone(),
            guarantee_display: guarantee_display.clone(),
        }),
        _ => None,
    };
    let state = build_state_summary(SummaryBuild {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &task,
        current_change_unit: change_unit,
        pending_user_judgment_refs,
        blocker_refs,
        write_ticket_summary: synthetic_write_ticket
            .as_ref()
            .map(|record| {
                write_ticket_summary_for_record(
                    None,
                    record,
                    planned_state_version,
                    None,
                    None,
                    guarantee_display.clone(),
                )
            })
            .transpose()?,
        evidence_summary,
        close_state: Some(close_state),
        close_blockers,
        guard_health: close_plan.guard_health,
        guarantee_display: guarantee_display.clone(),
    })?;
    let result = PrepareWriteResult {
        base: placeholder_base(),
        decision,
        state: Some(state),
        write_ticket_id: write_ticket_id.clone(),
        write_ticket_ref: write_ticket_ref.clone(),
        write_ticket,
        write_ticket_effect: if create_write_ticket {
            WriteTicketEffect::Issued
        } else {
            WriteTicketEffect::None
        },
        allowed_path_patterns,
        denied_path_patterns,
        control_surface,
        active_user_judgment_refs,
        write_decision_reasons: reasons.clone(),
        user_judgment_candidate: None,
        guarantee_display: guarantee_display.clone(),
    };

    let storage_mutations = if let Some(write_ticket_id) = &write_ticket_id {
        vec![CoreStorageMutation::InsertWriteTicket(WriteTicketInsert {
            write_ticket_id: write_ticket_id.as_str().to_owned(),
            task_id: task_id.as_str().to_owned(),
            change_unit_id: scope_change_unit_id.as_str().to_owned(),
            attempt_scope_json,
            created_by_actor_source: verified_invocation.actor_source.to_canonical_string(),
            created_by_judgment_id: None,
            expires_at,
            created_at,
            metadata_json: serde_json::to_string(&json!({
                "verification_basis": verified_invocation.verification_basis.clone()
            }))?,
        })]
    } else {
        Vec::new()
    };
    let event_kind = if allowed {
        "write_ticket_issued"
    } else {
        "write_decision_recorded"
    }
    .to_owned();
    let mut event_payload = object_from_value(json!({
        "task_id": task_id.clone(),
        "change_unit_id": branch_change_unit_id.clone(),
        "decision": decision,
        "write_ticket_id": write_ticket_id
            .as_ref()
            .map(|id| id.as_str().to_owned())
    }))?;
    if !allowed {
        event_payload.insert(
            "write_decision_reasons".to_owned(),
            serde_json::to_value(&reasons)?,
        );
    }

    Ok(PrepareWritePlan {
        task_id,
        change_unit_id: branch_change_unit_id,
        storage_mutations,
        event_kind,
        event_payload,
        result_fields: strip_base(serde_json::to_value(result)?)?,
        dry_run_summary: prepare_write_dry_run_summary(
            allowed,
            &reasons,
            write_ticket_ref,
            guarantee_display,
        ),
    })
}

fn effect_contract_reason(
    violation: EffectContractViolation,
    change_unit_ref: StateRecordRef,
) -> WriteDecisionReason {
    match violation {
        EffectContractViolation::FileWriteForbidden => write_decision_reason(
            WriteDecisionCategory::EffectContract,
            "effect_contract_forbids_product_file_write",
            "The current Change Unit effect contract forbids product-file writes.",
            vec![change_unit_ref],
        ),
        EffectContractViolation::FileWriteNotAllowed => write_decision_reason(
            WriteDecisionCategory::EffectContract,
            "effect_contract_effect_not_allowed",
            "The current Change Unit effect contract does not allow product-file writes.",
            vec![change_unit_ref],
        ),
        EffectContractViolation::PathNotAllowed => write_decision_reason(
            WriteDecisionCategory::EffectContract,
            "effect_contract_path_not_allowed",
            "One or more intended paths are outside the current Change Unit effect contract allowed paths.",
            vec![change_unit_ref],
        ),
    }
}

fn denied_write_ticket_paths(
    reasons: &[WriteDecisionReason],
    normalized_paths: &[String],
) -> Vec<String> {
    let path_denied = reasons.iter().any(|reason| {
        matches!(
            reason.code.as_str(),
            "path_out_of_scope"
                | "effect_contract_path_not_allowed"
                | "effect_contract_forbids_product_file_write"
                | "effect_contract_effect_not_allowed"
        )
    });
    if path_denied {
        normalized_paths.to_vec()
    } else {
        Vec::new()
    }
}
