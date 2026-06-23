use super::*;

impl CoreService {
    /// Executes `harness.record_run` through the shared Core mutation pipeline.
    pub fn record_run(
        &self,
        request: RecordRunRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        if let Some(envelope_task_id) = request.envelope.task_id.as_ref() {
            if envelope_task_id != &request.task_id {
                return validation_rejected(
                    request.envelope.dry_run,
                    None,
                    "task_id",
                    "envelope.task_id must match RecordRunRequest.task_id",
                );
            }
        }
        let prepared = match prepare_or_response(
            self,
            MethodName::RecordRun,
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
        let plan = match plan_record_run(
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
                        "run",
                        "would_record",
                        "Record run would create one Run and any compatible evidence or artifact links.",
                        Vec::new(),
                    ),
                },
            );
        }

        self.execute_prepared_request(
            prepared,
            OwnerPipelineBranch::CommitMutation {
                result_fields: plan.result_fields,
                event_kind: "run_recorded".to_owned(),
                event_payload: plan.event_payload,
                task_id: Some(plan.task_id),
                change_unit_id: plan.change_unit_id,
                storage_mutations: plan.storage_mutations,
            },
        )
    }
}

struct RecordRunArtifactPlan {
    artifact_ref: ArtifactRef,
    claim: Option<String>,
    source_mutation: Option<CoreStorageMutation>,
    run_link: CoreStorageMutation,
}

struct RecordRunArtifactContext<'a> {
    store: &'a CoreProjectStore,
    project_state: &'a ProjectStateHeader,
    request: &'a RecordRunRequest,
    verified_surface: &'a VerifiedSurfaceContext,
    run_id: &'a RunId,
    run_ref: &'a StateRecordRef,
    now: &'a UtcTimestamp,
}

fn plan_record_run(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: RecordRunRequest,
    verified_surface: &VerifiedSurfaceContext,
) -> Result<MethodPlan, PlanError> {
    if request.summary.trim().is_empty() {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "summary",
            "summary must not be empty",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    if request
        .run_id
        .as_ref()
        .is_some_and(|id| id.as_str().trim().is_empty())
    {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "run_id",
            "run_id must be null or a non-empty identifier",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }

    let normalized_changed_paths = match normalize_product_paths(
        &store.project_record().repo_root,
        &request.observed_changes.changed_paths,
    ) {
        Ok(paths) => sorted_unique(paths),
        Err(ProductPathError::Invalid) => {
            validation_plan_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "observed_changes.changed_paths",
                "changed_paths must be relative Product Repository paths that stay inside the repository",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
        Err(ProductPathError::LocalAccess) => {
            let response = rejected_pipeline_response(
                request.envelope.dry_run,
                Some(project_state.state_version),
                vec![tool_error(
                    ErrorCode::LocalAccessMismatch,
                    "changed_paths resolve outside the Product Repository",
                    false,
                    None,
                )],
            )
            .map_err(PlanError::Core)?;
            return Err(PlanError::Response(Box::new(response)));
        }
    };
    if request.observed_changes.product_file_write_observed && normalized_changed_paths.is_empty() {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "observed_changes",
            "product_file_write_observed requires at least one changed_path",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    if !request.observed_changes.product_file_write_observed && !normalized_changed_paths.is_empty()
    {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "observed_changes",
            "changed_paths require product_file_write_observed=true",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    if request
        .observed_changes
        .baseline_ref
        .as_ref()
        .is_some_and(|baseline_ref| baseline_ref != &request.baseline_ref)
    {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "observed_changes.baseline_ref",
            "observed_changes.baseline_ref must match request baseline_ref when present",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }

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
    let change_unit = store
        .change_unit_record(&request.task_id, request.change_unit_id.as_str())
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .ok_or_else(|| {
            PlanError::Response(Box::new(no_active_change_unit_response(
                &request.envelope,
                Some(project_state.state_version),
                "change_unit_id does not identify a Change Unit for the Task",
            )))
        })?;
    if change_unit.status != "active" || !change_unit.is_current {
        return Err(PlanError::Response(Box::new(
            no_active_change_unit_response(
                &request.envelope,
                Some(project_state.state_version),
                "record_run requires the current active Change Unit",
            ),
        )));
    }
    if !baseline_matches(&change_unit, &task, &request.baseline_ref)? {
        return Err(PlanError::Response(Box::new(baseline_stale_response(
            &request.envelope,
            Some(project_state.state_version),
            &request.baseline_ref,
        ))));
    }

    let planned_state_version = project_state.state_version + 1;
    let plan_now = utc_timestamp(service.now());
    let run_id = match request.run_id.clone().into_option() {
        Some(run_id) => run_id,
        None => allocate_run_id(service, store).map_err(PlanError::Core)?,
    };
    if request.run_id.is_some()
        && store.run_id_exists(run_id.as_str()).map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
    {
        let response = validation_rejected(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "run_id",
            "run_id already identifies an existing Run",
        )
        .map_err(PlanError::Core)?;
        return Err(PlanError::Response(Box::new(response)));
    }
    let run_ref = state_ref(
        StateRecordKind::Run,
        run_id.as_str(),
        &request.envelope.project_id,
        Some(&request.task_id),
        Some(planned_state_version),
    );
    let normalized_observed_changes = ObservedChanges {
        changed_paths: normalized_changed_paths.clone(),
        product_file_write_observed: request.observed_changes.product_file_write_observed,
        sensitive_categories: normalized_string_set(&request.observed_changes.sensitive_categories),
        baseline_ref: Some(request.baseline_ref.clone()).into(),
    };

    let artifact_context = RecordRunArtifactContext {
        store,
        project_state,
        request: &request,
        verified_surface,
        run_id: &run_id,
        run_ref: &run_ref,
        now: &plan_now,
    };
    let artifact_plans = plan_record_run_artifacts(service, artifact_context)?;
    let registered_artifacts = artifact_plans
        .iter()
        .map(|plan| plan.artifact_ref.clone())
        .collect::<Vec<_>>();

    let authorization_scope = if request.observed_changes.product_file_write_observed {
        let Some(write_authorization_id) = request.write_authorization_id.as_ref() else {
            return Err(PlanError::Response(Box::new(
                write_authorization_required_response(
                    &request.envelope,
                    Some(project_state.state_version),
                ),
            )));
        };
        let record = store
            .write_authorization_record(write_authorization_id.as_str())
            .map_err(|error| {
                PlanError::Response(Box::new(store_error_response(
                    &request.envelope,
                    project_state,
                    error,
                )))
            })?
            .ok_or_else(|| {
                PlanError::Response(Box::new(write_authorization_invalid_response(
                    &request.envelope,
                    Some(project_state.state_version),
                    "missing",
                    "write_authorization_id does not identify a Write Authorization",
                )))
            })?;
        let scope = validate_write_authorization_for_run(
            store,
            project_state,
            &request,
            &record,
            &normalized_observed_changes,
            *plan_now.as_datetime(),
        )?;
        Some((record, scope))
    } else {
        if request.write_authorization_id.is_some() {
            return Err(PlanError::Response(Box::new(
                write_authorization_invalid_response(
                    &request.envelope,
                    Some(project_state.state_version),
                    "incompatible",
                    "write_authorization_id is only consumed for observed product-file writes",
                ),
            )));
        }
        None
    };

    let evidence_summary = build_record_run_evidence_summary(
        &request,
        &run_ref,
        &registered_artifacts,
        &artifact_plans,
    );
    let evidence_summary_id = if evidence_summary.is_some() {
        Some(allocate_evidence_summary_id(service, store).map_err(PlanError::Core)?)
    } else {
        None
    };
    let evidence_summary_ref = evidence_summary_id.as_ref().map(|id| {
        state_ref(
            StateRecordKind::EvidenceSummary,
            id,
            &request.envelope.project_id,
            Some(&request.task_id),
            Some(planned_state_version),
        )
    });
    let close_basis_revision = task.close_basis_revision + 1;
    let close_basis_context = RecordRunCloseBasisContext {
        service,
        store,
        project_state,
        request: &request,
        task: &task,
        run_ref: &run_ref,
        authorization_scope: authorization_scope.as_ref(),
        evidence_summary_ref: evidence_summary_ref.clone(),
        registered_artifacts: &registered_artifacts,
        close_basis_revision,
        snapshot_state_version: planned_state_version,
        now: &plan_now,
    };
    let current_close_basis = build_record_run_close_basis(close_basis_context)?;
    let close_basis_json = current_close_basis
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
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
    let pending_user_judgment_refs = pending_refs_after_record_run_invalidation(
        store,
        project_state,
        &request,
        planned_state_version,
    )?;
    let guarantee_display =
        guarantee_display_for_surface(store, verified_surface, planned_state_version)?;
    let write_authority_summary = if let Some((record, _scope)) = &authorization_scope {
        let mut consumed_record = record.clone();
        consumed_record.status = storage_value(WriteAuthorizationStatus::Consumed)?;
        Some(write_authority_summary_for_record(
            &consumed_record,
            planned_state_version,
            Some(*plan_now.as_datetime()),
            Some(guarantee_display.clone()),
        )?)
    } else {
        projected_write_authority_summary(
            store,
            &request.task_id,
            planned_state_version,
            *plan_now.as_datetime(),
            Some(guarantee_display.clone()),
        )?
    };
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
            task.clone(),
            Some(change_unit.clone()),
            current_close_basis.clone(),
            pending_user_judgment_refs.clone(),
            blocker_refs.clone(),
            evidence_summary.clone(),
        ),
        *plan_now.as_datetime(),
    )?;
    let state = build_state_summary(SummaryBuild {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &task,
        current_change_unit: Some(&change_unit),
        pending_user_judgment_refs,
        blocker_refs: blocker_refs.clone(),
        write_authority_summary,
        evidence_summary: evidence_summary.clone(),
        close_state: Some(close_plan.close_state),
        close_blockers: close_plan.blockers,
        guarantee_display: Some(guarantee_display),
    })?;

    let run_summary = RunSummary {
        run_ref: run_ref.clone(),
        kind: request.kind,
        summary: request.summary.clone(),
        observed_changes: normalized_observed_changes.clone(),
        artifact_refs: registered_artifacts.clone(),
    };
    let result = RecordRunResult {
        base: placeholder_base(),
        run_summary,
        registered_artifacts: registered_artifacts.clone(),
        evidence_summary: evidence_summary.clone(),
        current_close_basis: current_close_basis.clone(),
        blocker_refs,
        state,
    };

    let mut storage_mutations = vec![CoreStorageMutation::InsertRun(RunInsert {
        run_id: run_id.as_str().to_owned(),
        task_id: request.task_id.as_str().to_owned(),
        change_unit_id: Some(request.change_unit_id.as_str().to_owned()),
        scope_revision: task.scope_revision,
        write_authorization_id: request
            .write_authorization_id
            .as_ref()
            .map(|id| id.as_str().to_owned()),
        kind: storage_value(request.kind)?,
        status: "recorded".to_owned(),
        summary_json: serde_json::to_string(&json!({
            "summary": request.summary
        }))?,
        observed_changes_json: serde_json::to_string(&normalized_observed_changes)?,
        evidence_updates_json: serde_json::to_string(&request.evidence_updates)?,
        authorization_effect_json: serde_json::to_string(&json!({
            "write_authorization_id": request.write_authorization_id,
            "effect": if authorization_scope.is_some() { "consumed" } else { "none" }
        }))?,
        created_by_surface_id: verified_surface.surface_id.as_str().to_owned(),
        created_by_surface_instance_id: verified_surface.surface_instance_id.as_str().to_owned(),
        metadata_json: serde_json::to_string(&json!({
            "verification_basis": verified_surface.verification_basis.clone()
        }))?,
    })];
    storage_mutations.push(CoreStorageMutation::UpdateTaskCloseBasis(
        TaskCloseBasisUpdate {
            task_id: request.task_id.as_str().to_owned(),
            close_basis_revision,
            close_basis_json,
        },
    ));
    storage_mutations.push(CoreStorageMutation::MarkUserJudgmentsSupersededOrStale(
        UserJudgmentInvalidation {
            task_id: request.task_id.as_str().to_owned(),
            judgment_kinds: vec![
                storage_value(JudgmentKind::FinalAcceptance)?,
                storage_value(JudgmentKind::ResidualRiskAcceptance)?,
            ],
        },
    ));
    if let Some((record, _scope)) = &authorization_scope {
        storage_mutations.push(CoreStorageMutation::ConsumeWriteAuthorization(
            WriteAuthorizationConsumption {
                write_authorization_id: record.write_authorization_id.clone(),
                run_id: run_id.as_str().to_owned(),
                expected_basis_state_version: record.basis_state_version,
            },
        ));
    }
    for plan in &artifact_plans {
        if let Some(mutation) = &plan.source_mutation {
            storage_mutations.push(mutation.clone());
        }
        storage_mutations.push(plan.run_link.clone());
    }
    if let (Some(evidence_summary), Some(evidence_summary_id)) =
        (&evidence_summary, evidence_summary_id.as_ref())
    {
        storage_mutations.push(CoreStorageMutation::UpsertEvidenceSummary(
            EvidenceSummaryUpsert {
                evidence_summary_id: evidence_summary_id.clone(),
                task_id: request.task_id.as_str().to_owned(),
                change_unit_id: Some(request.change_unit_id.as_str().to_owned()),
                status: storage_value(evidence_summary.status)?,
                coverage_json: serde_json::to_string(&evidence_summary.coverage_items)?,
                supporting_refs_json: serde_json::to_string(
                    &evidence_summary
                        .coverage_items
                        .iter()
                        .flat_map(|item| item.supporting_refs.clone())
                        .collect::<Vec<_>>(),
                )?,
                gap_refs_json: serde_json::to_string(
                    &evidence_summary
                        .coverage_items
                        .iter()
                        .flat_map(|item| item.gap_refs.clone())
                        .collect::<Vec<_>>(),
                )?,
                metadata_json: serde_json::to_string(&json!({
                    "updated_by_run_id": run_id.as_str()
                }))?,
            },
        ));
        for artifact_ref in &registered_artifacts {
            storage_mutations.push(CoreStorageMutation::LinkArtifact(ArtifactLinkInsert {
                artifact_id: artifact_ref.artifact_id.as_str().to_owned(),
                task_id: request.task_id.as_str().to_owned(),
                owner_record_kind: "evidence_summary".to_owned(),
                owner_record_id: evidence_summary_id.clone(),
                created_by_run_id: run_id.as_str().to_owned(),
                metadata_json: serde_json::to_string(&json!({
                    "relation": "evidence_support"
                }))?,
            }));
        }
    }

    let residual_risk_ids = current_close_basis
        .as_ref()
        .map(|basis| {
            basis
                .residual_risks
                .iter()
                .map(|risk| risk.risk_id.as_str().to_owned())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let event_payload = object_from_value(json!({
        "task_id": request.task_id,
        "change_unit_id": request.change_unit_id,
        "run_id": run_id,
        "source_run_ref": run_ref,
        "scope_revision": task.scope_revision,
        "close_basis_revision": close_basis_revision,
        "residual_risk_ids": residual_risk_ids,
        "kind": request.kind,
        "product_file_write_observed": normalized_observed_changes.product_file_write_observed,
        "write_authorization_id": authorization_scope
            .as_ref()
            .map(|(record, _scope)| record.write_authorization_id.clone()),
        "artifact_ids": registered_artifacts
            .iter()
            .map(|artifact| artifact.artifact_id.as_str().to_owned())
            .collect::<Vec<_>>()
    }))?;

    Ok(MethodPlan {
        task_id: request.task_id,
        change_unit_id: Some(request.change_unit_id),
        storage_mutations,
        event_payload,
        result_fields: strip_base(serde_json::to_value(result)?)?,
        next_actions: Vec::new(),
    })
}

fn pending_refs_after_record_run_invalidation(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &RecordRunRequest,
    planned_state_version: u64,
) -> Result<Vec<StateRecordRef>, PlanError> {
    let invalidated_kinds = BTreeSet::from([
        storage_value(JudgmentKind::FinalAcceptance)?,
        storage_value(JudgmentKind::ResidualRiskAcceptance)?,
    ]);
    let mut refs = Vec::new();
    for record_ref in store
        .pending_user_judgment_refs(&request.task_id, planned_state_version)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
    {
        let record = store
            .user_judgment_record(&record_ref.record_id)
            .map_err(|error| {
                PlanError::Response(Box::new(store_error_response(
                    &request.envelope,
                    project_state,
                    error,
                )))
            })?;
        if record
            .as_ref()
            .is_some_and(|record| invalidated_kinds.contains(&record.judgment_kind))
        {
            continue;
        }
        refs.push(state_ref_from_stored(record_ref));
    }
    Ok(refs)
}

struct RecordRunCloseBasisContext<'a> {
    service: &'a CoreService,
    store: &'a CoreProjectStore,
    project_state: &'a ProjectStateHeader,
    request: &'a RecordRunRequest,
    task: &'a TaskRecord,
    run_ref: &'a StateRecordRef,
    authorization_scope: Option<&'a (WriteAuthorizationRecord, AuthorizedAttemptScope)>,
    evidence_summary_ref: Option<StateRecordRef>,
    registered_artifacts: &'a [ArtifactRef],
    close_basis_revision: u64,
    snapshot_state_version: u64,
    now: &'a UtcTimestamp,
}

struct CloseBasisRefResolutionContext<'a> {
    store: &'a CoreProjectStore,
    project_state: &'a ProjectStateHeader,
    request: &'a RecordRunRequest,
    current_scope_revision: u64,
    field: &'static str,
    run_ref: &'a StateRecordRef,
    evidence_summary_ref: Option<&'a StateRecordRef>,
    registered_artifacts: &'a [ArtifactRef],
    snapshot_state_version: u64,
}

fn build_record_run_close_basis(
    context: RecordRunCloseBasisContext<'_>,
) -> Result<Option<CurrentCloseBasis>, PlanError> {
    let RecordRunCloseBasisContext {
        service,
        store,
        project_state,
        request,
        task,
        run_ref,
        authorization_scope,
        evidence_summary_ref,
        registered_artifacts,
        close_basis_revision,
        snapshot_state_version,
        now,
    } = context;
    let Some(assessment) = request.close_assessment.as_ref() else {
        return Ok(None);
    };
    if assessment.result_summary.trim().is_empty() {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "close_assessment.result_summary",
            "close_assessment.result_summary must not be empty",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }

    let mut result_refs = assessment.result_refs.clone();
    result_refs.push(run_ref.clone());
    result_refs.push(canonical_close_basis_ref(
        request,
        StateRecordKind::ChangeUnit,
        request.change_unit_id.as_str(),
        snapshot_state_version,
    ));
    if let Some(ref evidence_summary_ref) = evidence_summary_ref {
        result_refs.push(evidence_summary_ref.clone());
    }
    let result_refs = canonicalize_close_basis_refs(
        CloseBasisRefResolutionContext {
            store,
            project_state,
            request,
            current_scope_revision: task.scope_revision,
            field: "close_assessment.result_refs",
            run_ref,
            evidence_summary_ref: evidence_summary_ref.as_ref(),
            registered_artifacts,
            snapshot_state_version,
        },
        &result_refs,
    )?;

    if request.envelope.dry_run {
        for risk in &assessment.residual_risks {
            validate_residual_risk_input(
                CloseBasisRefResolutionContext {
                    store,
                    project_state,
                    request,
                    current_scope_revision: task.scope_revision,
                    field: "close_assessment.residual_risks[].source_refs",
                    run_ref,
                    evidence_summary_ref: evidence_summary_ref.as_ref(),
                    registered_artifacts,
                    snapshot_state_version,
                },
                risk,
            )?;
        }
        return Ok(None);
    }

    let mut allocated_risk_ids = BTreeSet::new();
    let mut residual_risks = Vec::new();
    for risk in &assessment.residual_risks {
        let source_refs = validate_residual_risk_input(
            CloseBasisRefResolutionContext {
                store,
                project_state,
                request,
                current_scope_revision: task.scope_revision,
                field: "close_assessment.residual_risks[].source_refs",
                run_ref,
                evidence_summary_ref: evidence_summary_ref.as_ref(),
                registered_artifacts,
                snapshot_state_version,
            },
            risk,
        )?;
        let risk_id = allocate_risk_id(service, &allocated_risk_ids).map_err(PlanError::Core)?;
        allocated_risk_ids.insert(risk_id.as_str().to_owned());
        residual_risks.push(ResidualRisk {
            risk_id,
            summary: normalize_display_text(&risk.summary),
            consequence: normalize_display_text(&risk.consequence),
            acceptance_required: risk.acceptance_required,
            source_refs,
        });
    }
    let sensitive_action_requirements = current_sensitive_action_requirements(
        store,
        project_state,
        request,
        task,
        run_ref,
        authorization_scope,
    )?;
    let derived_sensitive_categories = sensitive_category_summary(&sensitive_action_requirements);
    let caller_sensitive_categories = normalize_string_list(&assessment.sensitive_categories);
    if caller_sensitive_categories != derived_sensitive_categories {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "close_assessment.sensitive_categories",
            "close_assessment.sensitive_categories must match Core-derived sensitive requirements",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }

    Ok(Some(CurrentCloseBasis {
        close_basis_revision,
        scope_revision: task.scope_revision,
        task_id: request.task_id.clone(),
        change_unit_id: request.change_unit_id.clone(),
        baseline_ref: Some(request.baseline_ref.clone()).into(),
        result_summary: normalize_display_text(&assessment.result_summary),
        result_refs,
        evidence_summary_ref: evidence_summary_ref.into(),
        residual_risks,
        sensitive_categories: derived_sensitive_categories,
        sensitive_action_requirements,
        recovery_constraints: normalize_string_list(&assessment.recovery_constraints),
        source_run_ref: run_ref.clone(),
        updated_at: now.clone(),
    }))
}

fn current_sensitive_action_requirements(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &RecordRunRequest,
    task: &TaskRecord,
    run_ref: &StateRecordRef,
    authorization_scope: Option<&(WriteAuthorizationRecord, AuthorizedAttemptScope)>,
) -> Result<Vec<SensitiveActionRequirement>, PlanError> {
    let mut requirements =
        previous_current_sensitive_action_requirements(store, project_state, request, task)?;
    if let Some((record, scope)) = authorization_scope {
        if let Some(requirement) =
            sensitive_action_requirement_from_authorization(store, run_ref, record, scope)?
        {
            requirements.push(requirement);
        }
    }
    sorted_unique_sensitive_requirements(requirements)
}

fn previous_current_sensitive_action_requirements(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &RecordRunRequest,
    task: &TaskRecord,
) -> Result<Vec<SensitiveActionRequirement>, PlanError> {
    let task_revision = store
        .task_revision_record(&request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
    let Some(previous_basis) = task_revision.and_then(|record| record.current_close_basis) else {
        return Ok(Vec::new());
    };
    if previous_basis.task_id == request.task_id
        && previous_basis.change_unit_id == request.change_unit_id
        && previous_basis.scope_revision == task.scope_revision
        && previous_basis.close_basis_revision == task.close_basis_revision
        && previous_basis.baseline_ref.as_ref() == Some(&request.baseline_ref)
    {
        Ok(previous_basis.sensitive_action_requirements)
    } else {
        Ok(Vec::new())
    }
}

fn sensitive_action_requirement_from_authorization(
    store: &CoreProjectStore,
    run_ref: &StateRecordRef,
    record: &WriteAuthorizationRecord,
    scope: &AuthorizedAttemptScope,
) -> Result<Option<SensitiveActionRequirement>, PlanError> {
    let sensitive_categories = normalized_string_set(&scope.sensitive_categories);
    if sensitive_categories.is_empty() {
        return Ok(None);
    }
    let action_kind = scope.intended_operation.trim().to_owned();
    if action_kind.is_empty() {
        return Err(PlanError::Core(CorePipelineError::Store(
            StoreError::corrupt_owner_state_json(
                "write_authorizations",
                record.write_authorization_id.clone(),
                "attempt_scope_json",
            ),
        )));
    }
    let normalized_paths =
        normalize_product_paths(&store.project_record().repo_root, &scope.intended_paths).map_err(
            |_| {
                PlanError::Core(CorePipelineError::Store(
                    StoreError::corrupt_owner_state_json(
                        "write_authorizations",
                        record.write_authorization_id.clone(),
                        "attempt_scope_json",
                    ),
                ))
            },
        )?;
    if normalized_paths.is_empty() {
        return Err(PlanError::Core(CorePipelineError::Store(
            StoreError::corrupt_owner_state_json(
                "write_authorizations",
                record.write_authorization_id.clone(),
                "attempt_scope_json",
            ),
        )));
    }
    Ok(Some(SensitiveActionRequirement {
        action_kind,
        normalized_paths,
        sensitive_categories,
        baseline_ref: scope.baseline_ref.clone().into(),
        change_unit_id: scope.change_unit_id.clone(),
        source_run_ref: run_ref.clone(),
        source_write_authorization_ref: write_authorization_ref(
            record,
            run_ref
                .state_version
                .as_ref()
                .copied()
                .unwrap_or(record.basis_state_version),
        ),
    }))
}

fn sorted_unique_sensitive_requirements(
    requirements: Vec<SensitiveActionRequirement>,
) -> Result<Vec<SensitiveActionRequirement>, PlanError> {
    let mut unique = BTreeMap::new();
    for requirement in requirements {
        unique
            .entry(sensitive_requirement_key(&requirement)?)
            .or_insert(requirement);
    }
    Ok(unique.into_values().collect())
}

fn sensitive_requirement_key(
    requirement: &SensitiveActionRequirement,
) -> Result<(String, String, String, Option<String>, String), PlanError> {
    Ok((
        requirement.action_kind.clone(),
        serde_json::to_string(&requirement.normalized_paths)?,
        serde_json::to_string(&requirement.sensitive_categories)?,
        requirement
            .baseline_ref
            .as_ref()
            .map(|baseline_ref| baseline_ref.as_str().to_owned()),
        requirement.change_unit_id.as_str().to_owned(),
    ))
}

fn sensitive_category_summary(requirements: &[SensitiveActionRequirement]) -> Vec<String> {
    requirements
        .iter()
        .flat_map(|requirement| requirement.sensitive_categories.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn validate_residual_risk_input(
    context: CloseBasisRefResolutionContext<'_>,
    risk: &volicord_types::ResidualRiskInput,
) -> Result<Vec<StateRecordRef>, PlanError> {
    let request = context.request;
    let project_state = context.project_state;
    if risk.summary.trim().is_empty() {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "close_assessment.residual_risks.summary",
            "residual risk summary must not be empty",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    if risk.consequence.trim().is_empty() {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "close_assessment.residual_risks.consequence",
            "residual risk consequence must not be empty",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    canonicalize_close_basis_refs(context, &risk.source_refs)
}

fn canonicalize_close_basis_refs(
    context: CloseBasisRefResolutionContext<'_>,
    refs: &[StateRecordRef],
) -> Result<Vec<StateRecordRef>, PlanError> {
    let mut normalized = BTreeMap::new();
    for record_ref in refs {
        let normalized_ref = resolve_close_basis_ref(&context, record_ref)?;
        let key = close_basis_ref_identity_key(&normalized_ref);
        if let Some(previous) = normalized.get(&key) {
            if previous != &normalized_ref {
                validation_plan_error(
                    context.request.envelope.dry_run,
                    Some(context.project_state.state_version),
                    context.field,
                    "duplicate close assessment refs must resolve to the same canonical record",
                )?;
                unreachable!("validation_plan_error always returns Err");
            }
        } else {
            normalized.insert(key, normalized_ref);
        }
    }
    Ok(normalized.into_values().collect())
}

fn resolve_close_basis_ref(
    context: &CloseBasisRefResolutionContext<'_>,
    record_ref: &StateRecordRef,
) -> Result<StateRecordRef, PlanError> {
    let request = context.request;
    let project_state = context.project_state;
    if record_ref.record_id.as_str().trim().is_empty() {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            context.field,
            "close assessment refs must use non-empty record_id values",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    if !matches!(
        record_ref.record_kind,
        StateRecordKind::Run
            | StateRecordKind::Artifact
            | StateRecordKind::EvidenceSummary
            | StateRecordKind::ChangeUnit
    ) {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            context.field,
            "close assessment refs may only use run, artifact, evidence_summary, or change_unit record_kind",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    if record_ref.project_id != request.envelope.project_id {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            context.field,
            "close assessment refs must belong to the request project",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    if record_ref.task_id.as_ref() != Some(&request.task_id) {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            context.field,
            "close assessment refs must belong to the request Task",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }

    match record_ref.record_kind {
        StateRecordKind::Run => resolve_close_basis_run_ref(context, record_ref),
        StateRecordKind::ChangeUnit => resolve_close_basis_change_unit_ref(context, record_ref),
        StateRecordKind::EvidenceSummary => {
            resolve_close_basis_evidence_summary_ref(context, record_ref)
        }
        StateRecordKind::Artifact => resolve_close_basis_artifact_ref(context, record_ref),
        _ => unreachable!("unsupported close-basis record kind rejected above"),
    }
}

fn resolve_close_basis_run_ref(
    context: &CloseBasisRefResolutionContext<'_>,
    record_ref: &StateRecordRef,
) -> Result<StateRecordRef, PlanError> {
    let request = context.request;
    if record_ref.record_id == context.run_ref.record_id {
        return Ok(context.run_ref.clone());
    }
    let record = context
        .store
        .run_record(record_ref.record_id.as_str())
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                context.project_state,
                error,
            )))
        })?;
    let compatible = match record.as_ref() {
        Some(record) => run_record_is_close_basis_compatible(context, record)?,
        None => false,
    };
    if !compatible {
        validation_plan_error(
            request.envelope.dry_run,
            Some(context.project_state.state_version),
            context.field,
            "Run refs in close_assessment must exist for the request Task, current Change Unit, current scope revision, and current baseline",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    let record = record.expect("compatible run record is present");
    Ok(canonical_close_basis_ref(
        request,
        StateRecordKind::Run,
        &record.run_id,
        context.snapshot_state_version,
    ))
}

fn run_record_is_close_basis_compatible(
    context: &CloseBasisRefResolutionContext<'_>,
    record: &RunRecord,
) -> Result<bool, PlanError> {
    let Some(change_unit_id) = record.change_unit_id.as_deref() else {
        return Ok(false);
    };
    if record.project_id != context.request.envelope.project_id.as_str()
        || record.task_id != context.request.task_id.as_str()
        || change_unit_id != context.request.change_unit_id.as_str()
        || record.scope_revision != context.current_scope_revision
        || record.baseline_ref.as_deref() != Some(context.request.baseline_ref.as_str())
        || record.status != "recorded"
    {
        return Ok(false);
    }
    Ok(context
        .store
        .current_change_unit(&context.request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &context.request.envelope,
                context.project_state,
                error,
            )))
        })?
        .as_ref()
        .is_some_and(|record| {
            record.change_unit_id == change_unit_id
                && record.status == "active"
                && record.is_current
        }))
}

fn resolve_close_basis_change_unit_ref(
    context: &CloseBasisRefResolutionContext<'_>,
    record_ref: &StateRecordRef,
) -> Result<StateRecordRef, PlanError> {
    let request = context.request;
    let record = context
        .store
        .change_unit_record(&request.task_id, record_ref.record_id.as_str())
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                context.project_state,
                error,
            )))
        })?;
    if record.as_ref().is_none_or(|record| {
        record.project_id != request.envelope.project_id.as_str()
            || record.task_id != request.task_id.as_str()
            || record.change_unit_id != request.change_unit_id.as_str()
            || record.status != "active"
            || !record.is_current
    }) {
        validation_plan_error(
            request.envelope.dry_run,
            Some(context.project_state.state_version),
            context.field,
            "Change Unit refs in close_assessment must identify the current Change Unit",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    let record = record.expect("current Change Unit record is present");
    Ok(canonical_close_basis_ref(
        request,
        StateRecordKind::ChangeUnit,
        &record.change_unit_id,
        context.snapshot_state_version,
    ))
}

fn resolve_close_basis_evidence_summary_ref(
    context: &CloseBasisRefResolutionContext<'_>,
    record_ref: &StateRecordRef,
) -> Result<StateRecordRef, PlanError> {
    let request = context.request;
    if context
        .evidence_summary_ref
        .is_some_and(|summary_ref| summary_ref.record_id == record_ref.record_id)
    {
        return Ok(context
            .evidence_summary_ref
            .expect("checked evidence summary ref is present")
            .clone());
    }
    let record = context
        .store
        .evidence_summary_record(record_ref.record_id.as_str())
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                context.project_state,
                error,
            )))
        })?;
    let latest = context
        .store
        .latest_evidence_summary(&request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                context.project_state,
                error,
            )))
        })?;
    if record.as_ref().is_none_or(|record| {
        record.project_id != request.envelope.project_id.as_str()
            || record.task_id != request.task_id.as_str()
            || latest
                .as_ref()
                .is_none_or(|latest| latest.evidence_summary_id != record.evidence_summary_id)
    }) {
        validation_plan_error(
            request.envelope.dry_run,
            Some(context.project_state.state_version),
            context.field,
            "Evidence Summary refs in close_assessment must identify the current Task evidence summary",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    let record = record.expect("current Evidence Summary record is present");
    Ok(canonical_close_basis_ref(
        request,
        StateRecordKind::EvidenceSummary,
        &record.evidence_summary_id,
        context.snapshot_state_version,
    ))
}

fn resolve_close_basis_artifact_ref(
    context: &CloseBasisRefResolutionContext<'_>,
    record_ref: &StateRecordRef,
) -> Result<StateRecordRef, PlanError> {
    let request = context.request;
    if context
        .registered_artifacts
        .iter()
        .any(|artifact| artifact.artifact_id.as_str() == record_ref.record_id.as_str())
    {
        return Ok(canonical_close_basis_ref(
            request,
            StateRecordKind::Artifact,
            record_ref.record_id.as_str(),
            context.snapshot_state_version,
        ));
    }
    let record = context
        .store
        .artifact_record(record_ref.record_id.as_str())
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                context.project_state,
                error,
            )))
        })?;
    let owner_link_exists = context
        .store
        .artifact_has_task_owner_link(record_ref.record_id.as_str(), request.task_id.as_str())
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                context.project_state,
                error,
            )))
        })?;
    if record
        .as_ref()
        .map(|record| {
            let available = persistent_artifact_is_verified_current(context.store, record)?;
            Ok::<_, CorePipelineError>(
                record.project_id == request.envelope.project_id.as_str()
                    && record.task_id == request.task_id.as_str()
                    && available
                    && owner_link_exists,
            )
        })
        .transpose()?
        .unwrap_or(false)
    {
        let record = record.expect("verified artifact record is present");
        Ok(canonical_close_basis_ref(
            request,
            StateRecordKind::Artifact,
            &record.artifact_id,
            context.snapshot_state_version,
        ))
    } else {
        validation_plan_error(
            request.envelope.dry_run,
            Some(context.project_state.state_version),
            context.field,
            "Artifact refs in close_assessment must identify verified available artifacts owned by the request Task",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
}

fn canonical_close_basis_ref(
    request: &RecordRunRequest,
    record_kind: StateRecordKind,
    record_id: &str,
    snapshot_state_version: u64,
) -> StateRecordRef {
    state_ref(
        record_kind,
        record_id,
        &request.envelope.project_id,
        Some(&request.task_id),
        Some(snapshot_state_version),
    )
}

fn close_basis_ref_identity_key(record_ref: &StateRecordRef) -> (String, String, String, String) {
    (
        storage_value(record_ref.record_kind).unwrap_or_else(|_| "unknown".to_owned()),
        record_ref.record_id.as_str().to_owned(),
        record_ref.project_id.as_str().to_owned(),
        record_ref
            .task_id
            .as_ref()
            .map(|task_id| task_id.as_str().to_owned())
            .unwrap_or_default(),
    )
}

fn normalize_display_text(value: &str) -> String {
    value.trim().to_owned()
}

fn normalize_string_list(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| normalize_display_text(value))
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn plan_record_run_artifacts(
    service: &CoreService,
    context: RecordRunArtifactContext<'_>,
) -> Result<Vec<RecordRunArtifactPlan>, PlanError> {
    let request = context.request;
    let project_state = context.project_state;
    let mut input_ids = BTreeSet::new();
    let mut staged_handles = BTreeSet::new();
    let mut plans = Vec::new();
    for input in &request.artifact_inputs {
        if input.artifact_input_id.as_str().trim().is_empty() {
            return artifact_input_validation_plan_error(
                request,
                project_state,
                input,
                "staged_handle_not_found",
                "artifact_input_id must not be empty",
            );
        }
        if !input_ids.insert(input.artifact_input_id.as_str()) {
            return artifact_input_validation_plan_error(
                request,
                project_state,
                input,
                "staged_handle_not_found",
                "artifact_input_id values must be unique within one request",
            );
        }
        match input.source_kind {
            ArtifactInputSourceKind::StagedArtifact => {
                if input.staged_artifact_handle.is_none() || input.existing_artifact_ref.is_some() {
                    return artifact_input_validation_plan_error(
                        request,
                        project_state,
                        input,
                        "staged_handle_not_found",
                        "staged_artifact inputs must populate only staged_artifact_handle",
                    );
                }
                let handle = input
                    .staged_artifact_handle
                    .as_ref()
                    .expect("checked staged_artifact_handle above");
                if !staged_handles.insert(handle.handle_id.as_str()) {
                    return artifact_input_validation_plan_error(
                        request,
                        project_state,
                        input,
                        "staged_handle_consumed",
                        "a staged artifact handle can be consumed at most once",
                    );
                }
                plans.push(plan_staged_artifact_input(
                    service, &context, input, handle,
                )?);
            }
            ArtifactInputSourceKind::ExistingArtifact => {
                if input.existing_artifact_ref.is_none() || input.staged_artifact_handle.is_some() {
                    return artifact_input_validation_plan_error(
                        request,
                        project_state,
                        input,
                        "staged_handle_not_found",
                        "existing_artifact inputs must populate only existing_artifact_ref",
                    );
                }
                plans.push(plan_existing_artifact_input(
                    &context,
                    input,
                    input
                        .existing_artifact_ref
                        .as_ref()
                        .expect("checked existing_artifact_ref above"),
                )?);
            }
        }
    }
    Ok(plans)
}

fn plan_staged_artifact_input(
    service: &CoreService,
    context: &RecordRunArtifactContext<'_>,
    input: &ArtifactInput,
    handle: &StagedArtifactHandle,
) -> Result<RecordRunArtifactPlan, PlanError> {
    let store = context.store;
    let project_state = context.project_state;
    let request = context.request;
    let verified_surface = context.verified_surface;
    let run_id = context.run_id;
    let run_ref = context.run_ref;
    if handle.project_id != request.envelope.project_id {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_project_mismatch",
            "staged artifact handle belongs to a different project",
        );
    }
    if handle.task_id != request.task_id {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_task_mismatch",
            "staged artifact handle belongs to a different Task",
        );
    }
    if handle.consumed {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_consumed",
            "staged artifact handle is already consumed",
        );
    }

    let record = store
        .artifact_staging_record(handle.handle_id.as_str())
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .ok_or_else(|| {
            PlanError::Response(Box::new(artifact_input_validation_response(
                request,
                project_state,
                input,
                "staged_handle_not_found",
                "staged artifact handle cannot be found",
            )))
        })?;
    let stored_expires_at = validate_staged_artifact_record(
        project_state,
        request,
        verified_surface,
        input,
        handle,
        &record,
        context.now,
    )?;

    let artifact_id = allocate_artifact_id(service, store).map_err(PlanError::Core)?;
    let uri = format!(
        "harness-artifact://{}/{}",
        request.envelope.project_id.as_str(),
        artifact_id.as_str()
    );
    let display_name = staged_artifact_display_name(&record);
    let content_type = record
        .content_type
        .clone()
        .unwrap_or_else(|| handle.content_type.clone());
    let sha256 = record
        .sha256
        .clone()
        .expect("staged artifact validation ensures sha256 is present");
    let size_bytes = record
        .size_bytes
        .expect("staged artifact validation ensures size_bytes is present");
    let redaction_state =
        parse_storage_value("artifact_staging.redaction_state", &record.redaction_state)?;
    let artifact_ref = ArtifactRef {
        artifact_id: artifact_id.clone(),
        project_id: request.envelope.project_id.clone(),
        task_id: request.task_id.clone(),
        display_name: display_name.clone(),
        content_type: Some(content_type.clone()).into(),
        sha256: Some(sha256.clone()).into(),
        size_bytes: Some(size_bytes).into(),
        integrity_status: ArtifactIntegrityStatus::Verified,
        redaction_state,
        availability: ArtifactAvailability::Available,
        created_by_run_ref: Some(run_ref.clone()).into(),
        created_by_surface_id: Some(SurfaceId::new(record.created_by_surface_id.clone())).into(),
        created_by_surface_instance_id: Some(SurfaceInstanceId::new(
            record.created_by_surface_instance_id.clone(),
        ))
        .into(),
        storage_ref: Some(StorageRef::new(uri.clone())).into(),
    };
    let source_mutation = Some(CoreStorageMutation::PromoteStagedArtifact(
        ArtifactPromotion {
            handle_id: handle.handle_id.as_str().to_owned(),
            artifact_id: artifact_id.as_str().to_owned(),
            task_id: request.task_id.as_str().to_owned(),
            run_id: run_id.as_str().to_owned(),
            expected_created_by_surface_id: verified_surface.surface_id.as_str().to_owned(),
            expected_created_by_surface_instance_id: verified_surface
                .surface_instance_id
                .as_str()
                .to_owned(),
            expected_sha256: sha256,
            expected_size_bytes: size_bytes,
            expected_redaction_state: record.redaction_state.clone(),
            expected_expires_at: stored_expires_at.to_string(),
            uri,
            retention_json: "{}".to_owned(),
            producer_json: serde_json::to_string(&json!({
                "display_name": display_name,
                "content_type": content_type,
                "created_by_surface_id": verified_surface.surface_id.as_str(),
                "created_by_surface_instance_id": verified_surface.surface_instance_id.as_str(),
                "artifact_input_id": input.artifact_input_id.as_str(),
                "relation_hint": input.relation_hint,
                "claim": input.claim
            }))?,
            metadata_json: serde_json::to_string(&json!({
                "source_kind": "staged_artifact"
            }))?,
        },
    ));
    let run_link = CoreStorageMutation::LinkArtifact(ArtifactLinkInsert {
        artifact_id: artifact_id.as_str().to_owned(),
        task_id: request.task_id.as_str().to_owned(),
        owner_record_kind: "run".to_owned(),
        owner_record_id: run_id.as_str().to_owned(),
        created_by_run_id: run_id.as_str().to_owned(),
        metadata_json: artifact_link_metadata(input)?,
    });

    Ok(RecordRunArtifactPlan {
        artifact_ref,
        claim: input.claim.as_ref().cloned(),
        source_mutation,
        run_link,
    })
}

fn validate_staged_artifact_record(
    project_state: &ProjectStateHeader,
    request: &RecordRunRequest,
    verified_surface: &VerifiedSurfaceContext,
    input: &ArtifactInput,
    handle: &StagedArtifactHandle,
    record: &StoredArtifactStagingRecord,
    now: &UtcTimestamp,
) -> Result<UtcTimestamp, PlanError> {
    if record.project_id != request.envelope.project_id.as_str() {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_project_mismatch",
            "stored staged artifact belongs to a different project",
        );
    }
    if record.task_id != request.task_id.as_str() {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_task_mismatch",
            "stored staged artifact belongs to a different Task",
        );
    }
    if record.created_by_surface_id != verified_surface.surface_id.as_str()
        || record.created_by_surface_instance_id != verified_surface.surface_instance_id.as_str()
        || handle.created_by_surface_id.as_str() != record.created_by_surface_id
        || handle.created_by_surface_instance_id.as_str() != record.created_by_surface_instance_id
    {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_surface_mismatch",
            "staged artifact provenance does not match the verified surface",
        );
    }
    if record.status == "consumed" {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_consumed",
            "staged artifact handle is already consumed",
        );
    }
    let stored_expires_at: UtcTimestamp = parse_owner_storage_value(
        "artifact_staging",
        record.handle_id.clone(),
        "expires_at",
        &record.expires_at,
    )?;
    if record.status == "expired" || now >= &stored_expires_at {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_expired",
            "staged artifact handle is expired",
        );
    }
    if stored_expires_at != handle.expires_at {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_checksum_mismatch",
            "staged artifact expiration does not match the submitted handle",
        );
    }
    if record.status != "staged" {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_not_found",
            "staged artifact handle is not consumable",
        );
    }
    if record.sha256.as_deref() != Some(handle.sha256.as_str())
        || input
            .expected_sha256
            .as_deref()
            .is_some_and(|expected| record.sha256.as_deref() != Some(expected))
        || record.sha256.is_none()
    {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_checksum_mismatch",
            "staged artifact checksum does not match the submitted handle or expectation",
        );
    }
    if record.size_bytes != Some(handle.size_bytes)
        || input
            .expected_size_bytes
            .is_some_and(|expected| record.size_bytes != Some(expected))
        || record.size_bytes.is_none()
    {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_size_mismatch",
            "staged artifact size does not match the submitted handle or expectation",
        );
    }
    let expected_redaction = input.redaction_state.unwrap_or(handle.redaction_state);
    if record.redaction_state != redaction_state_value(handle.redaction_state)
        || record.redaction_state != redaction_state_value(expected_redaction)
    {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_checksum_mismatch",
            "staged artifact redaction_state does not match the submitted handle or expectation",
        );
    }
    if record.content_type.as_deref() != Some(handle.content_type.as_str()) {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_checksum_mismatch",
            "staged artifact content_type does not match the submitted handle",
        );
    }
    Ok(stored_expires_at)
}

fn plan_existing_artifact_input(
    context: &RecordRunArtifactContext<'_>,
    input: &ArtifactInput,
    existing_ref: &ArtifactRef,
) -> Result<RecordRunArtifactPlan, PlanError> {
    let store = context.store;
    let project_state = context.project_state;
    let request = context.request;
    let run_id = context.run_id;
    if existing_ref.project_id != request.envelope.project_id
        || existing_ref.task_id != request.task_id
    {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_project_mismatch",
            "existing artifact ref must belong to the request project and Task",
        );
    }
    let record = store
        .artifact_record(existing_ref.artifact_id.as_str())
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .ok_or_else(|| {
            PlanError::Response(Box::new(artifact_missing_response(
                request,
                project_state,
                "existing artifact cannot be found",
            )))
        })?;
    let artifact_available = persistent_artifact_is_verified_current(store, &record)?;
    if record.task_id != request.task_id.as_str()
        || record.project_id != request.envelope.project_id.as_str()
        || !artifact_available
        || !store
            .artifact_has_task_owner_link(
                existing_ref.artifact_id.as_str(),
                request.task_id.as_str(),
            )
            .map_err(|error| {
                PlanError::Response(Box::new(store_error_response(
                    &request.envelope,
                    project_state,
                    error,
                )))
            })?
    {
        return Err(PlanError::Response(Box::new(artifact_missing_response(
            request,
            project_state,
            "existing artifact is not available for this Task",
        ))));
    }
    if existing_ref.integrity_status != ArtifactIntegrityStatus::Verified {
        return Err(PlanError::Response(Box::new(artifact_missing_response(
            request,
            project_state,
            "existing artifact does not have verified integrity facts",
        ))));
    }
    let Some(existing_sha256) = existing_ref.sha256.as_ref() else {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_checksum_mismatch",
            "verified existing artifact refs must include sha256",
        );
    };
    let Some(existing_size_bytes) = existing_ref.size_bytes.as_ref().copied() else {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_size_mismatch",
            "verified existing artifact refs must include size_bytes",
        );
    };
    let Some(existing_content_type) = existing_ref.content_type.as_ref() else {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_checksum_mismatch",
            "verified existing artifact refs must include content_type",
        );
    };
    if record.sha256.as_deref() != Some(existing_sha256.as_str())
        || input
            .expected_sha256
            .as_deref()
            .is_some_and(|expected| record.sha256.as_deref() != Some(expected))
    {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_checksum_mismatch",
            "existing artifact checksum does not match the stored artifact",
        );
    }
    if record.size_bytes != Some(existing_size_bytes)
        || input
            .expected_size_bytes
            .is_some_and(|expected| record.size_bytes != Some(expected))
    {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_size_mismatch",
            "existing artifact size does not match the stored artifact",
        );
    }
    if record.content_type.as_deref() != Some(existing_content_type.as_str()) {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_checksum_mismatch",
            "existing artifact content_type does not match the stored artifact",
        );
    }
    let stored_redaction_state: RedactionState = parse_owner_storage_value(
        "artifacts",
        record.artifact_id.clone(),
        "redaction_state",
        &record.redaction_state,
    )?;
    let expected_redaction = input
        .redaction_state
        .unwrap_or(existing_ref.redaction_state);
    if stored_redaction_state != existing_ref.redaction_state
        || stored_redaction_state != expected_redaction
    {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_checksum_mismatch",
            "existing artifact redaction_state does not match the stored artifact",
        );
    }
    let artifact_ref = artifact_ref_from_verified_record(
        store,
        &record,
        Some(existing_ref.display_name.clone()),
        None,
    )?;
    let run_link = CoreStorageMutation::LinkArtifact(ArtifactLinkInsert {
        artifact_id: existing_ref.artifact_id.as_str().to_owned(),
        task_id: request.task_id.as_str().to_owned(),
        owner_record_kind: "run".to_owned(),
        owner_record_id: run_id.as_str().to_owned(),
        created_by_run_id: run_id.as_str().to_owned(),
        metadata_json: artifact_link_metadata(input)?,
    });
    Ok(RecordRunArtifactPlan {
        artifact_ref,
        claim: input.claim.as_ref().cloned(),
        source_mutation: None,
        run_link,
    })
}

fn validate_write_authorization_for_run(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &RecordRunRequest,
    record: &WriteAuthorizationRecord,
    observed_changes: &ObservedChanges,
    now: DateTime<Utc>,
) -> Result<AuthorizedAttemptScope, PlanError> {
    if record.status == "consumed" || record.status == "revoked" {
        let reason = if record.status == "consumed" {
            "consumed"
        } else {
            "revoked"
        };
        return Err(PlanError::Response(Box::new(
            write_authorization_invalid_response(
                &request.envelope,
                Some(project_state.state_version),
                reason,
                "Write Authorization is not active",
            ),
        )));
    }
    if record.basis_state_version != project_state.state_version {
        return Err(PlanError::Response(Box::new(
            stale_write_authorization_basis_response(
                &request.envelope,
                record,
                project_state.state_version,
            ),
        )));
    }
    if record.status != "active" {
        let reason = match record.status.as_str() {
            "consumed" => "consumed",
            "expired" => "expired",
            "stale" => "stale",
            "revoked" => "revoked",
            _ => "incompatible",
        };
        return Err(PlanError::Response(Box::new(
            write_authorization_invalid_response(
                &request.envelope,
                Some(project_state.state_version),
                reason,
                "Write Authorization is not active",
            ),
        )));
    }
    if write_authorization_is_expired(record, now).map_err(CorePipelineError::from)? {
        return Err(PlanError::Response(Box::new(
            write_authorization_invalid_response(
                &request.envelope,
                Some(project_state.state_version),
                "expired",
                "Write Authorization is expired",
            ),
        )));
    }
    let scope: AuthorizedAttemptScope = decode_required_json::<PersistedAuthorizedAttemptScope>(
        "write_authorizations",
        record.write_authorization_id.clone(),
        "attempt_scope_json",
        Some(&record.attempt_scope_json),
    )?
    .into();
    let scope_paths =
        normalize_product_paths(&store.project_record().repo_root, &scope.intended_paths).map_err(
            |_| {
                PlanError::Core(CorePipelineError::Store(
                    StoreError::corrupt_owner_state_json(
                        "write_authorizations",
                        record.write_authorization_id.clone(),
                        "attempt_scope_json",
                    ),
                ))
            },
        )?;
    if record.task_id != request.task_id.as_str()
        || record.change_unit_id.as_deref() != Some(request.change_unit_id.as_str())
        || scope.task_id != request.task_id
        || scope.change_unit_id != request.change_unit_id
        || !scope.product_file_write_intended
        || scope.baseline_ref.as_ref() != Some(&request.baseline_ref)
        || string_set(&normalized_string_set(&scope.sensitive_categories))
            != string_set(&observed_changes.sensitive_categories)
        || !paths_are_authorized(&observed_changes.changed_paths, &scope_paths)
    {
        return Err(PlanError::Response(Box::new(
            write_authorization_invalid_response(
                &request.envelope,
                Some(project_state.state_version),
                "incompatible",
                "Write Authorization is not compatible with the recorded run",
            ),
        )));
    }
    Ok(scope)
}

fn build_record_run_evidence_summary(
    request: &RecordRunRequest,
    run_ref: &StateRecordRef,
    registered_artifacts: &[ArtifactRef],
    artifact_plans: &[RecordRunArtifactPlan],
) -> Option<volicord_types::EvidenceSummary> {
    if request.evidence_updates.is_empty() {
        return None;
    }
    let mut coverage_items = Vec::new();
    for update in &request.evidence_updates {
        let mut item = update.clone();
        if !item.supporting_refs.iter().any(|record_ref| {
            record_ref.record_kind == StateRecordKind::Run
                && record_ref.record_id == run_ref.record_id
        }) {
            item.supporting_refs.push(run_ref.clone());
        }
        for plan in artifact_plans {
            if plan.claim.as_deref() == Some(update.claim.as_str())
                && !item
                    .supporting_artifact_refs
                    .iter()
                    .any(|artifact| artifact.artifact_id == plan.artifact_ref.artifact_id)
            {
                item.supporting_artifact_refs
                    .push(plan.artifact_ref.clone());
            }
        }
        coverage_items.push(item);
    }
    let artifact_refs = unique_artifact_refs(
        registered_artifacts
            .iter()
            .cloned()
            .chain(
                coverage_items
                    .iter()
                    .flat_map(|item| item.supporting_artifact_refs.clone()),
            )
            .collect(),
    );
    let required_claims = coverage_items
        .iter()
        .filter(|item| item.required_for_close)
        .map(|item| item.claim.clone())
        .collect::<Vec<_>>();
    let status = evidence_status_for_items(&coverage_items);
    Some(volicord_types::EvidenceSummary {
        status,
        completion_policy: CompletionPolicy {
            evidence_required: !required_claims.is_empty(),
            required_claims,
        },
        coverage_items,
        artifact_refs,
        updated_by_run_ref: Some(run_ref.clone()),
    })
}

fn staged_artifact_display_name(record: &StoredArtifactStagingRecord) -> String {
    string_member(
        &display_only_json_object_lossy(&record.artifact_json),
        "display_name",
    )
    .unwrap_or_else(|| record.handle_id.clone())
}

fn artifact_link_metadata(input: &ArtifactInput) -> CoreResult<String> {
    Ok(serde_json::to_string(&json!({
        "artifact_input_id": input.artifact_input_id.as_str(),
        "source_kind": input.source_kind,
        "relation_hint": input.relation_hint,
        "claim": input.claim
    }))?)
}
