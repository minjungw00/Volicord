use super::*;

impl CoreService {
    /// Executes `harness.prepare_write` through the shared Core mutation pipeline.
    pub fn prepare_write(
        &self,
        request: PrepareWriteRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        if let Some(envelope_task_id) = &request.envelope.task_id {
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
        .or_else(|| request.envelope.task_id.clone())
        .map(TaskRequirement::Exact)
        .unwrap_or(TaskRequirement::Required);

    if request.envelope.dry_run {
        MethodPolicy::exact(
            request.requested_access_class(),
            task,
            ReplayPolicy::None,
            FreshnessPolicy::IfPresent,
            MethodEffectPolicy::DryRunPreview,
        )
    } else {
        MethodPolicy::exact(
            request.requested_access_class(),
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
    verified_surface: &VerifiedSurfaceContext,
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
                    ErrorCode::LocalAccessMismatch,
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
    }

    let pending_user_judgment_refs = store
        .pending_user_judgment_refs(&task_id, project_state.state_version)
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
    if !pending_user_judgment_refs.is_empty() {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::UserJudgment,
            "user_judgment_unresolved",
            "A user-owned judgment required before write preparation remains unresolved.",
            pending_user_judgment_refs.clone(),
        ));
    }

    let mut active_user_judgment_refs = Vec::new();
    if !request.sensitive_categories.is_empty() {
        let matching_sensitive_approval = matching_sensitive_approval(
            store,
            project_state,
            &request,
            &task_id,
            change_unit,
            &normalized_paths,
        )?;
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
                "A matching sensitive-action approval is required before Write Authorization.",
                Vec::new(),
            ));
        }
    }

    if verified_surface.access_class != AccessClass::WriteAuthorization {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::SurfaceCapability,
            "surface_access_class_mismatch",
            "The verified surface access class is incompatible with Write Authorization.",
            Vec::new(),
        ));
    }
    if !surface_supports_prepare_write(&verified_surface.capability_profile) {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::SurfaceCapability,
            "surface_capability_insufficient",
            "The verified surface lacks the write-authorization capability declaration.",
            Vec::new(),
        ));
    }

    let guarantee_display = Some(write_authorization_guarantee());
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
    let write_authorization_id =
        allocate_write_authorization_id(service, store).map_err(PlanError::Core)?;
    let authorized_attempt_scope = AuthorizedAttemptScope {
        task_id: task_id.clone(),
        change_unit_id: scope_change_unit_id.clone(),
        intended_operation: request.intended_operation.clone(),
        intended_paths: normalized_paths.clone(),
        product_file_write_intended: request.product_file_write_intended,
        sensitive_categories: request.sensitive_categories.clone(),
        baseline_ref: Some(request.baseline_ref.clone()),
    };
    let attempt_scope_json = serde_json::to_string(&authorized_attempt_scope)?;
    let expires_at = "2999-01-01T00:00:00Z".to_owned();
    let write_authorization_ref = allowed.then(|| {
        state_ref(
            StateRecordKind::WriteAuthorization,
            write_authorization_id.as_str(),
            &request.envelope.project_id,
            Some(&task_id),
            Some(planned_state_version),
        )
    });
    let write_authorization = write_authorization_ref
        .as_ref()
        .map(|write_authorization_ref| WriteAuthorizationSummary {
            write_authorization_ref: write_authorization_ref.clone(),
            status: WriteAuthorizationStatus::Active,
            authorized_attempt_scope: authorized_attempt_scope.clone(),
            basis_state_version: planned_state_version,
            expires_at: Some(expires_at.clone()),
        });
    let synthetic_write_authorization = allowed.then(|| WriteAuthorizationRecord {
        project_id: request.envelope.project_id.as_str().to_owned(),
        write_authorization_id: write_authorization_id.as_str().to_owned(),
        task_id: task_id.as_str().to_owned(),
        change_unit_id: Some(scope_change_unit_id.as_str().to_owned()),
        basis_state_version: planned_state_version,
        status: "active".to_owned(),
        attempt_scope_json: attempt_scope_json.clone(),
        expires_at: expires_at.clone(),
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
    let state = build_state_summary(SummaryBuild {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &task,
        current_change_unit: change_unit,
        pending_user_judgment_refs,
        blocker_refs,
        active_write_authorization: synthetic_write_authorization.as_ref(),
        options: SummaryOptions::prepare_write(),
    })?;
    let result = PrepareWriteResult {
        base: placeholder_base(),
        decision,
        state: Some(state),
        write_authorization_ref: write_authorization_ref.clone(),
        write_authorization,
        authorization_effect: if allowed {
            AuthorizationEffect::Created
        } else {
            AuthorizationEffect::None
        },
        active_user_judgment_refs,
        write_decision_reasons: reasons.clone(),
        user_judgment_candidate: None,
        guarantee_display: guarantee_display.clone(),
    };

    let storage_mutations = if allowed {
        vec![CoreStorageMutation::InsertWriteAuthorization(
            WriteAuthorizationInsert {
                write_authorization_id: write_authorization_id.as_str().to_owned(),
                task_id: task_id.as_str().to_owned(),
                change_unit_id: scope_change_unit_id.as_str().to_owned(),
                attempt_scope_json,
                created_by_surface_id: verified_surface.surface_id.as_str().to_owned(),
                created_by_surface_instance_id: verified_surface
                    .surface_instance_id
                    .as_str()
                    .to_owned(),
                created_by_judgment_id: None,
                expires_at,
                metadata_json: serde_json::to_string(&json!({
                    "verification_basis": verified_surface.verification_basis.clone()
                }))?,
            },
        )]
    } else {
        Vec::new()
    };
    let event_kind = if allowed {
        "write_authorization_created"
    } else {
        "write_decision_recorded"
    }
    .to_owned();
    let mut event_payload = object_from_value(json!({
        "task_id": task_id.clone(),
        "change_unit_id": branch_change_unit_id.clone(),
        "decision": decision,
        "write_authorization_id": allowed.then(|| write_authorization_id.as_str().to_owned())
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
            write_authorization_ref,
            guarantee_display,
        ),
    })
}
