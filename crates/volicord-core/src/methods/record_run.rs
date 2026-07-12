use super::*;

impl CoreService {
    /// Executes `volicord.record_run` through the shared Core mutation pipeline.
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
                request.operation_category(),
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
    evidence_target: Option<EvidenceTarget>,
    source_mutation: Option<CoreStorageMutation>,
    run_link: CoreStorageMutation,
}

struct RecordRunObservationPlan {
    observation: EvidenceObservation,
    observation_ref: StateRecordRef,
    mutation: CoreStorageMutation,
}

struct RecordRunArtifactContext<'a> {
    store: &'a CoreProjectStore,
    project_state: &'a ProjectStateHeader,
    request: &'a RecordRunRequest,
    verified_invocation: &'a VerifiedInvocationContext,
    run_id: &'a RunId,
    run_ref: &'a StateRecordRef,
    now: &'a UtcTimestamp,
}

fn task_mode_allows_run_kind(task_mode: TaskMode, run_kind: RunKind) -> bool {
    match task_mode {
        TaskMode::Advisor => run_kind == RunKind::ShapingUpdate,
        TaskMode::Direct => run_kind == RunKind::Direct,
        TaskMode::Work => matches!(run_kind, RunKind::ShapingUpdate | RunKind::Implementation),
    }
}

fn normalize_record_run_evidence_targets(request: &mut RecordRunRequest) {
    for update in &mut request.evidence_updates {
        normalize_evidence_target(&mut update.target);
    }
    for observation in &mut request.evidence_observations {
        normalize_evidence_target(&mut observation.target);
    }
    for artifact in &mut request.artifact_inputs {
        if let Some(target) = artifact.evidence_target.as_mut() {
            normalize_evidence_target(target);
        }
    }
}

fn normalize_evidence_target(target: &mut EvidenceTarget) {
    if let EvidenceTarget::SupplementalClaim { statement, .. } = target {
        *statement = normalize_display_text(statement);
    }
}

fn validate_record_run_evidence_targets(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &RecordRunRequest,
) -> Result<Vec<CoreStorageMutation>, PlanError> {
    let mut supplemental = BTreeMap::<String, String>::new();
    let mut validate_target = |target: &EvidenceTarget, field: &'static str| {
        match target {
            EvidenceTarget::AcceptanceCriterion {
                acceptance_criterion_id,
            } => {
                if acceptance_criterion_id.as_str().trim().is_empty() {
                    return validation_plan_error(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        field,
                        "acceptance criterion evidence target ID must not be empty",
                    );
                }
                let record = store
                    .acceptance_criterion_record(acceptance_criterion_id.as_str())
                    .map_err(CorePipelineError::from)?;
                let Some(record) = record else {
                    return validation_plan_error(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        field,
                        "acceptance criterion evidence target is unknown",
                    );
                };
                if record.task_id != request.task_id.as_str() || record.status != "active" {
                    return validation_plan_error(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        field,
                        "acceptance criterion evidence target must be current for this Task",
                    );
                }
            }
            EvidenceTarget::SupplementalClaim {
                evidence_claim_id,
                statement,
            } => {
                if evidence_claim_id.as_str().trim().is_empty() || statement.is_empty() {
                    return validation_plan_error(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        field,
                        "supplemental evidence targets require a non-empty ID and statement",
                    );
                }
                if let Some(existing) =
                    supplemental.insert(evidence_claim_id.as_str().to_owned(), statement.clone())
                {
                    if existing != *statement {
                        return validation_plan_error(
                            request.envelope.dry_run,
                            Some(project_state.state_version),
                            field,
                            "one supplemental evidence claim ID cannot use multiple statements",
                        );
                    }
                }
            }
        }
        Ok(())
    };

    for update in &request.evidence_updates {
        validate_target(&update.target, "evidence_updates[].target")?;
        if update.coverage_state == EvidenceCoverageUpdateState::NotApplicable {
            if let EvidenceTarget::AcceptanceCriterion {
                acceptance_criterion_id,
            } = &update.target
            {
                let record = store
                    .acceptance_criterion_record(acceptance_criterion_id.as_str())
                    .map_err(CorePipelineError::from)?
                    .expect("target validation ensures the criterion exists");
                let requirement: EvidenceRequirement = parse_owner_storage_value(
                    "acceptance_criteria",
                    record.acceptance_criterion_id,
                    "evidence_requirement",
                    &record.evidence_requirement,
                )?;
                if requirement == EvidenceRequirement::Required {
                    validation_plan_error(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        "evidence_updates[].coverage_state",
                        "required acceptance criteria cannot be marked not_applicable",
                    )?;
                    unreachable!("validation_plan_error always returns Err");
                }
            }
        }
    }
    for observation in &request.evidence_observations {
        validate_target(&observation.target, "evidence_observations[].target")?;
    }
    for artifact in &request.artifact_inputs {
        if let Some(target) = artifact.evidence_target.as_ref() {
            validate_target(target, "artifact_inputs[].evidence_target")?;
        }
    }

    let mut mutations = Vec::new();
    for (evidence_claim_id, statement) in supplemental {
        match store
            .evidence_claim_record(&request.task_id, &evidence_claim_id)
            .map_err(CorePipelineError::from)?
        {
            Some(record) if record.statement == statement => {}
            Some(_) => {
                validation_plan_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "evidence_target.statement",
                    "supplemental evidence claim statements are immutable within a Task",
                )?;
                unreachable!("validation_plan_error always returns Err");
            }
            None => mutations.push(CoreStorageMutation::EnsureEvidenceClaim(
                EvidenceClaimInsert {
                    evidence_claim_id,
                    task_id: request.task_id.as_str().to_owned(),
                    statement,
                },
            )),
        }
    }
    Ok(mutations)
}

fn plan_record_run(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    mut request: RecordRunRequest,
    verified_invocation: &VerifiedInvocationContext,
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
                    ErrorCode::InvocationContextMismatch,
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
    let task_mode = parse_task_mode(&task.mode)?;
    if !task_mode_allows_run_kind(task_mode, request.kind) {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "kind",
            "kind is not compatible with the current Task mode",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    if task_mode == TaskMode::Advisor
        && (request.observed_changes.product_file_write_observed
            || !normalized_changed_paths.is_empty())
    {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "observed_changes",
            "advisor Task runs cannot report Product Repository file changes",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    if task_mode == TaskMode::Advisor && request.write_ticket_id.is_some() {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "write_ticket_id",
            "advisor Task runs cannot consume a write ticket",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
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

    normalize_record_run_evidence_targets(&mut request);
    let evidence_claim_mutations =
        validate_record_run_evidence_targets(store, project_state, &request)?;

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
        verified_invocation,
        run_id: &run_id,
        run_ref: &run_ref,
        now: &plan_now,
    };
    let artifact_plans = plan_record_run_artifacts(service, artifact_context)?;
    let registered_artifacts = artifact_plans
        .iter()
        .map(|plan| plan.artifact_ref.clone())
        .collect::<Vec<_>>();
    let observation_context = RecordRunObservationContext {
        service,
        store,
        project_state,
        request: &request,
        verified_invocation,
        run_id: &run_id,
        run_ref: &run_ref,
        registered_artifacts: &registered_artifacts,
        artifact_plans: &artifact_plans,
        current_scope_revision: task.scope_revision,
        planned_state_version,
        now: &plan_now,
    };
    let observation_plans = plan_record_run_observations(&observation_context)?;
    let evidence_observations = observation_plans
        .iter()
        .map(|plan| plan.observation.clone())
        .collect::<Vec<_>>();
    let observation_refs_by_target = observation_refs_by_target(&observation_plans);

    let write_ticket_scope = if request.observed_changes.product_file_write_observed {
        let Some(write_ticket_id) = request.write_ticket_id.as_ref() else {
            return Err(PlanError::Response(Box::new(
                write_ticket_required_response(
                    &request.envelope,
                    Some(project_state.state_version),
                ),
            )));
        };
        let record = store
            .write_ticket_record(write_ticket_id.as_str())
            .map_err(|error| {
                PlanError::Response(Box::new(store_error_response(
                    &request.envelope,
                    project_state,
                    error,
                )))
            })?
            .ok_or_else(|| {
                PlanError::Response(Box::new(write_ticket_invalid_response(
                    &request.envelope,
                    Some(project_state.state_version),
                    "missing",
                    "write_ticket_id does not identify a write ticket",
                )))
            })?;
        let scope = validate_write_ticket_for_run(
            store,
            project_state,
            &request,
            &record,
            &normalized_observed_changes,
            *plan_now.as_datetime(),
        )?;
        Some((record, scope))
    } else {
        if request.write_ticket_id.is_some() {
            return Err(PlanError::Response(Box::new(
                write_ticket_invalid_response(
                    &request.envelope,
                    Some(project_state.state_version),
                    "incompatible",
                    "write_ticket_id is only consumed for observed product-file writes",
                ),
            )));
        }
        None
    };

    let acceptance_criteria = active_acceptance_criteria_for_task(store, &request.task_id)?;
    let mut recorded_evidence_summary = build_record_run_evidence_summary(
        &observation_context,
        &request,
        &run_ref,
        &registered_artifacts,
        &artifact_plans,
        &observation_refs_by_target,
    )?;
    let evidence_summary_id = if recorded_evidence_summary.is_some() {
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
        write_ticket_scope: write_ticket_scope.as_ref(),
        evidence_summary_ref: evidence_summary_ref.clone(),
        registered_artifacts: &registered_artifacts,
        close_basis_revision,
        snapshot_state_version: planned_state_version,
        now: &plan_now,
    };
    let current_close_basis = build_record_run_close_basis(close_basis_context)?;
    recorded_evidence_summary = recorded_evidence_summary
        .map(|summary| evidence_summary_for_display(summary, current_close_basis.as_ref()));
    let projected_close_evidence_summary = evidence_summary_with_required_criteria(
        recorded_evidence_summary.clone(),
        &acceptance_criteria,
    );
    let projected_state_evidence_summary = match recorded_evidence_summary.as_ref() {
        Some(_) => projected_close_evidence_summary.clone(),
        None => projected_evidence_summary_for_criteria(
            store,
            &request.envelope.project_id,
            planned_state_version,
            &task,
            &acceptance_criteria,
        )?
        .map(|summary| evidence_summary_for_display(summary, current_close_basis.as_ref())),
    };
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
    let pending_authorities = pending_judgment_authorities_for_plan(
        store,
        project_state,
        &request.envelope,
        &request.task_id,
    )?
    .into_iter()
    .filter(|authority| {
        !matches!(
            authority.judgment_kind,
            JudgmentKind::FinalAcceptance | JudgmentKind::ResidualRiskAcceptance
        )
    })
    .collect::<Vec<_>>();
    let lifecycle_phase = projected_judgment_lifecycle_phase(
        project_state,
        &task,
        Some(&change_unit),
        &pending_authorities,
    );
    let mut projected_task = task.clone();
    projected_task.close_basis_revision = close_basis_revision;
    if let Some(lifecycle_phase) = lifecycle_phase {
        projected_task.lifecycle_phase = lifecycle_phase.to_owned();
    }
    let guarantee_display =
        guarantee_display_for_invocation(store, verified_invocation, planned_state_version)?;
    let write_ticket_summary = if let Some((record, _scope)) = &write_ticket_scope {
        let mut consumed_record = record.clone();
        consumed_record.status = storage_value(WriteTicketStatus::Consumed)?;
        consumed_record.consumed_by_run_id = Some(run_id.as_str().to_owned());
        consumed_record.consumed_at = Some(plan_now.to_string());
        let observation_refs = observation_plans
            .iter()
            .map(|plan| plan.observation_ref.clone())
            .collect::<Vec<_>>();
        Some(write_ticket_summary_for_record(
            None,
            &consumed_record,
            planned_state_version,
            Some(*plan_now.as_datetime()),
            Some(observation_refs),
            Some(guarantee_display.clone()),
        )?)
    } else {
        projected_write_ticket_summary(
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
        verified_invocation,
        &request.envelope,
        &request.task_id,
        close_context_with_pending_authorities(
            close_context_with_projected_acceptance_criteria(
                close_context_with_record_run_projection(
                    close_context_from_projection(
                        projected_task.clone(),
                        Some(change_unit.clone()),
                        current_close_basis.clone(),
                        pending_user_judgment_refs.clone(),
                        blocker_refs.clone(),
                        projected_close_evidence_summary,
                    ),
                    run_ref.clone(),
                    evidence_observations.clone(),
                    registered_artifacts.clone(),
                ),
                &acceptance_criteria,
            ),
            pending_authorities,
        ),
        *plan_now.as_datetime(),
    )?;
    let state = build_state_summary(SummaryBuild {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &projected_task,
        current_change_unit: Some(&change_unit),
        acceptance_criteria,
        pending_user_judgment_refs,
        blocker_refs: blocker_refs.clone(),
        write_ticket_summary,
        evidence_summary: projected_state_evidence_summary,
        close_state: Some(close_plan.close_state),
        close_blockers: close_plan.blockers,
        guard_health: close_plan.guard_health,
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
        evidence_summary: recorded_evidence_summary.clone(),
        evidence_observations: evidence_observations.clone(),
        current_close_basis: current_close_basis.clone(),
        blocker_refs,
        state,
    };

    let mut storage_mutations = vec![CoreStorageMutation::InsertRun(RunInsert {
        run_id: run_id.as_str().to_owned(),
        task_id: request.task_id.as_str().to_owned(),
        change_unit_id: Some(request.change_unit_id.as_str().to_owned()),
        scope_revision: task.scope_revision,
        write_ticket_id: request
            .write_ticket_id
            .as_ref()
            .map(|id| id.as_str().to_owned()),
        kind: storage_value(request.kind)?,
        status: "recorded".to_owned(),
        summary_json: serde_json::to_string(&json!({
            "summary": request.summary
        }))?,
        observed_changes_json: serde_json::to_string(&normalized_observed_changes)?,
        evidence_updates_json: serde_json::to_string(&request.evidence_updates)?,
        write_ticket_effect_json: serde_json::to_string(&json!({
            "write_ticket_id": request.write_ticket_id,
            "effect": if write_ticket_scope.is_some() { "consumed" } else { "none" }
        }))?,
        created_by_actor_source: verified_invocation.actor_source.to_canonical_string(),
        metadata_json: serde_json::to_string(&json!({
            "verification_basis": verified_invocation.verification_basis.clone()
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
    if let Some(lifecycle_phase) = lifecycle_phase {
        storage_mutations.push(task_lifecycle_mutation(&request.task_id, lifecycle_phase));
    }
    if let Some((record, _scope)) = &write_ticket_scope {
        storage_mutations.push(CoreStorageMutation::ConsumeWriteTicket(
            WriteTicketConsumption {
                write_ticket_id: record.write_ticket_id.clone(),
                run_id: run_id.as_str().to_owned(),
                expected_basis_state_version: record.basis_state_version,
            },
        ));
    }
    storage_mutations.extend(evidence_claim_mutations);
    for plan in &artifact_plans {
        if let Some(mutation) = &plan.source_mutation {
            storage_mutations.push(mutation.clone());
        }
        storage_mutations.push(plan.run_link.clone());
    }
    for plan in &observation_plans {
        storage_mutations.push(plan.mutation.clone());
        for artifact_ref in &plan.observation.output_artifact_refs {
            storage_mutations.push(CoreStorageMutation::LinkArtifact(ArtifactLinkInsert {
                artifact_id: artifact_ref.artifact_id.as_str().to_owned(),
                task_id: request.task_id.as_str().to_owned(),
                owner_record_kind: "evidence_observation".to_owned(),
                owner_record_id: plan.observation.observation_id.as_str().to_owned(),
                created_by_run_id: run_id.as_str().to_owned(),
                metadata_json: serde_json::to_string(&json!({
                    "relation": "evidence_observation_output"
                }))?,
            }));
        }
    }
    if let (Some(evidence_summary), Some(evidence_summary_id)) =
        (&recorded_evidence_summary, evidence_summary_id.as_ref())
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
                        .flat_map(|item| item.supporting_run_refs.clone())
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
        "write_ticket_id": write_ticket_scope
            .as_ref()
            .map(|(record, _scope)| record.write_ticket_id.clone()),
        "artifact_ids": registered_artifacts
            .iter()
            .map(|artifact| artifact.artifact_id.as_str().to_owned())
            .collect::<Vec<_>>()
        ,
        "evidence_observation_ids": evidence_observations
            .iter()
            .map(|observation| observation.observation_id.as_str().to_owned())
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

struct RecordRunObservationContext<'a> {
    service: &'a CoreService,
    store: &'a CoreProjectStore,
    project_state: &'a ProjectStateHeader,
    request: &'a RecordRunRequest,
    verified_invocation: &'a VerifiedInvocationContext,
    run_id: &'a RunId,
    run_ref: &'a StateRecordRef,
    registered_artifacts: &'a [ArtifactRef],
    artifact_plans: &'a [RecordRunArtifactPlan],
    current_scope_revision: u64,
    planned_state_version: u64,
    now: &'a UtcTimestamp,
}

fn plan_record_run_observations(
    context: &RecordRunObservationContext<'_>,
) -> Result<Vec<RecordRunObservationPlan>, PlanError> {
    let mut plans = Vec::new();
    for input in &context.request.evidence_observations {
        plans.push(plan_record_run_observation(context, input)?);
    }
    let explicit_observation_targets = plans
        .iter()
        .map(|plan| plan.observation.target.clone())
        .collect::<BTreeSet<_>>();
    for update in &context.request.evidence_updates {
        validate_record_run_evidence_update(context, update, &explicit_observation_targets)?;
        if update.coverage_state == EvidenceCoverageUpdateState::Supported
            && !explicit_observation_targets.contains(&update.target)
        {
            if let Some(provenance) = update.provenance.as_ref() {
                plans.push(plan_record_run_observation(
                    context,
                    &observation_input_from_evidence_update(context, update, provenance),
                )?);
            } else {
                for input in reused_observation_inputs_for_update(context, update)? {
                    plans.push(plan_record_run_observation(context, &input)?);
                }
            }
        }
    }
    Ok(plans)
}

fn plan_record_run_observation(
    context: &RecordRunObservationContext<'_>,
    input: &EvidenceObservationInput,
) -> Result<RecordRunObservationPlan, PlanError> {
    validate_evidence_source_assurance(
        context.request.envelope.dry_run,
        Some(context.project_state.state_version),
        "evidence_observations[]",
        input.source_kind,
        input.assurance_level,
    )?;
    validate_evidence_observation_state_refs(
        context,
        "evidence_observations[].input_refs",
        &input.input_refs,
    )?;
    let source_refs = normalize_source_refs(
        context.store,
        context.project_state,
        &context.request.envelope,
        &context.request.task_id,
        "evidence_observations[].source_refs",
        &input.source_refs,
    )?;
    let canonical_output_artifact_refs = canonical_evidence_artifact_refs(
        context,
        "evidence_observations[].output_artifact_refs",
        &input.output_artifact_refs,
    )?;
    let mut canonical_input = input.clone();
    canonical_input.output_artifact_refs = canonical_output_artifact_refs;
    let input = &canonical_input;
    if input
        .tool_name
        .as_ref()
        .is_some_and(|value| value.trim().is_empty())
    {
        validation_plan_error(
            context.request.envelope.dry_run,
            Some(context.project_state.state_version),
            "evidence_observations[].tool_name",
            "tool_name must be null or a non-empty string",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    if input
        .tool_invocation_id
        .as_ref()
        .is_some_and(|value| value.trim().is_empty())
    {
        validation_plan_error(
            context.request.envelope.dry_run,
            Some(context.project_state.state_version),
            "evidence_observations[].tool_invocation_id",
            "tool_invocation_id must be null or a non-empty string",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }

    let observation_id = allocate_evidence_observation_id(context.service, context.store)
        .map_err(PlanError::Core)?;
    let observation_ref = state_ref(
        StateRecordKind::EvidenceObservation,
        observation_id.as_str(),
        &context.request.envelope.project_id,
        Some(&context.request.task_id),
        Some(context.planned_state_version),
    );
    let observed_by_actor_source = input
        .observed_by_actor_source
        .as_ref()
        .cloned()
        .unwrap_or_else(|| context.verified_invocation.actor_source.clone());
    let output_artifact_refs =
        unique_artifact_refs(output_artifact_refs_for_observation(context, input));
    let limitations = normalize_string_list(&input.limitations);
    let observation = EvidenceObservation {
        observation_id,
        project_id: context.request.envelope.project_id.clone(),
        task_id: context.request.task_id.clone(),
        change_unit_id: Some(context.request.change_unit_id.clone()).into(),
        run_ref: Some(context.run_ref.clone()).into(),
        target: input.target.clone(),
        source_kind: input.source_kind,
        assurance_level: input.assurance_level,
        observed_by_actor_source: Some(observed_by_actor_source).into(),
        tool_name: input.tool_name.clone(),
        tool_invocation_id: input.tool_invocation_id.clone(),
        tool_metadata: input.tool_metadata.clone(),
        input_refs: input.input_refs.clone(),
        source_refs,
        output_artifact_refs,
        limitations,
        observed_at: input.observed_at.clone(),
        recorded_at: context.now.clone(),
    };
    let mutation = CoreStorageMutation::InsertEvidenceObservation(EvidenceObservationInsert {
        evidence_observation_id: observation.observation_id.as_str().to_owned(),
        task_id: observation.task_id.as_str().to_owned(),
        change_unit_id: observation
            .change_unit_id
            .as_ref()
            .map(|id| id.as_str().to_owned()),
        run_id: Some(context.run_id.as_str().to_owned()),
        acceptance_criterion_id: match &observation.target {
            EvidenceTarget::AcceptanceCriterion {
                acceptance_criterion_id,
            } => Some(acceptance_criterion_id.as_str().to_owned()),
            EvidenceTarget::SupplementalClaim { .. } => None,
        },
        evidence_claim_id: match &observation.target {
            EvidenceTarget::SupplementalClaim {
                evidence_claim_id, ..
            } => Some(evidence_claim_id.as_str().to_owned()),
            EvidenceTarget::AcceptanceCriterion { .. } => None,
        },
        source_kind: storage_value(observation.source_kind)?,
        assurance_level: storage_value(observation.assurance_level)?,
        observed_by_actor_source: observation
            .observed_by_actor_source
            .as_ref()
            .map(ActorSource::to_canonical_string),
        tool_name: observation.tool_name.as_ref().cloned(),
        tool_invocation_id: observation.tool_invocation_id.as_ref().cloned(),
        tool_metadata_json: serde_json::to_string(&observation.tool_metadata)?,
        input_refs_json: serde_json::to_string(&observation.input_refs)?,
        source_refs_json: serde_json::to_string(&observation.source_refs)?,
        output_artifact_refs_json: serde_json::to_string(&observation.output_artifact_refs)?,
        limitations_json: serde_json::to_string(&observation.limitations)?,
        observed_at: observation.observed_at.to_canonical_string(),
        recorded_at: observation.recorded_at.to_canonical_string(),
        metadata_json: serde_json::to_string(&json!({
            "recorded_by_run_id": context.run_id.as_str(),
            "verification_basis": context.verified_invocation.verification_basis.clone()
        }))?,
    });
    Ok(RecordRunObservationPlan {
        observation,
        observation_ref,
        mutation,
    })
}

fn validate_record_run_evidence_update(
    context: &RecordRunObservationContext<'_>,
    update: &EvidenceCoverageUpdate,
    explicit_observation_targets: &BTreeSet<EvidenceTarget>,
) -> Result<(), PlanError> {
    validate_evidence_update_observation_refs(
        context,
        &update.target,
        &update.observation_refs,
        update.coverage_state == EvidenceCoverageUpdateState::Supported
            && !explicit_observation_targets.contains(&update.target)
            && update.provenance.is_none(),
    )?;
    validate_supporting_run_refs(context, &update.supporting_run_refs)?;
    canonical_evidence_artifact_refs(
        context,
        "evidence_updates[].supporting_artifact_refs",
        &update.supporting_artifact_refs,
    )?;
    validate_evidence_gap_refs(context, &update.gap_refs)?;
    if let Some(provenance) = update.provenance.as_ref() {
        validate_evidence_source_assurance(
            context.request.envelope.dry_run,
            Some(context.project_state.state_version),
            "evidence_updates[].provenance",
            provenance.source_kind,
            provenance.assurance_level,
        )?;
        if provenance
            .tool_name
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            validation_plan_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                "evidence_updates[].provenance.tool_name",
                "tool_name must be null or a non-empty string",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
        normalize_source_refs(
            context.store,
            context.project_state,
            &context.request.envelope,
            &context.request.task_id,
            "evidence_updates[].provenance.source_refs",
            &provenance.source_refs,
        )?;
        if provenance
            .tool_invocation_id
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            validation_plan_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                "evidence_updates[].provenance.tool_invocation_id",
                "tool_invocation_id must be null or a non-empty string",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
    }
    if update.coverage_state == EvidenceCoverageUpdateState::Supported
        && !explicit_observation_targets.contains(&update.target)
        && update.provenance.is_none()
        && update.observation_refs.is_empty()
    {
        validation_plan_error(
            context.request.envelope.dry_run,
            Some(context.project_state.state_version),
            "evidence_updates[].provenance",
            "supported evidence updates require provenance or a target-matching evidence observation",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    Ok(())
}

fn observation_input_from_evidence_update(
    context: &RecordRunObservationContext<'_>,
    update: &EvidenceCoverageUpdate,
    provenance: &EvidenceUpdateProvenance,
) -> EvidenceObservationInput {
    EvidenceObservationInput {
        target: update.target.clone(),
        source_kind: provenance.source_kind,
        assurance_level: provenance.assurance_level,
        observed_by_actor_source: None.into(),
        tool_name: provenance.tool_name.clone(),
        tool_invocation_id: provenance.tool_invocation_id.clone(),
        tool_metadata: provenance.tool_metadata.clone(),
        input_refs: update.supporting_run_refs.clone(),
        source_refs: provenance.source_refs.clone(),
        output_artifact_refs: update.supporting_artifact_refs.clone(),
        limitations: provenance.limitations.clone(),
        observed_at: provenance
            .observed_at
            .clone()
            .unwrap_or_else(|| context.now.clone()),
    }
}

fn validate_evidence_source_assurance(
    dry_run: bool,
    state_version: Option<u64>,
    field: &'static str,
    source_kind: EvidenceSourceKind,
    assurance_level: EvidenceAssuranceLevel,
) -> Result<(), PlanError> {
    if evidence_assurance_matches_source(source_kind, assurance_level) {
        Ok(())
    } else {
        validation_plan_error(
            dry_run,
            state_version,
            field,
            "evidence source_kind and assurance_level must describe the same provenance class",
        )
    }
}

fn validate_evidence_observation_state_refs(
    context: &RecordRunObservationContext<'_>,
    field: &'static str,
    refs: &[StateRecordRef],
) -> Result<(), PlanError> {
    for record_ref in refs {
        if record_ref.record_id.as_str().trim().is_empty() {
            validation_plan_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                field,
                "evidence observation refs must use non-empty record_id values",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
        if field == "evidence_updates[].observation_refs"
            && record_ref.record_kind != StateRecordKind::EvidenceObservation
        {
            validation_plan_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                field,
                "evidence update observation_refs must identify evidence_observation records",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
        if record_ref.project_id != context.request.envelope.project_id {
            validation_plan_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                field,
                "evidence observation refs must belong to the request project",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
        if record_ref
            .task_id
            .as_ref()
            .is_some_and(|task_id| task_id != &context.request.task_id)
        {
            validation_plan_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                field,
                "evidence observation refs must not belong to another Task",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
    }
    Ok(())
}

fn validate_evidence_update_observation_refs(
    context: &RecordRunObservationContext<'_>,
    target: &EvidenceTarget,
    refs: &[StateRecordRef],
    require_strong_reuse: bool,
) -> Result<(), PlanError> {
    for record_ref in refs {
        if record_ref.record_kind != StateRecordKind::EvidenceObservation
            || record_ref.project_id != context.request.envelope.project_id
            || record_ref.task_id.as_ref() != Some(&context.request.task_id)
            || record_ref.record_id.as_str().trim().is_empty()
        {
            validation_plan_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                "evidence_updates[].observation_refs",
                "evidence update observation refs must identify same-Task evidence observations",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
        let record = context
            .store
            .evidence_observation_record(record_ref.record_id.as_str())
            .map_err(CorePipelineError::from)?;
        let Some(record) = record else {
            validation_plan_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                "evidence_updates[].observation_refs",
                "evidence update observation refs must identify existing observations",
            )?;
            unreachable!("validation_plan_error always returns Err");
        };
        if record.task_id != context.request.task_id.as_str()
            || record.change_unit_id.as_deref() != Some(context.request.change_unit_id.as_str())
            || !evidence_observation_record_matches_target(&record, target)
        {
            validation_plan_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                "evidence_updates[].observation_refs",
                "evidence update observation refs must match the current Task, Change Unit, and evidence target",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
        let source_run = record
            .run_id
            .as_deref()
            .map(|run_id| context.store.run_record(run_id))
            .transpose()
            .map_err(CorePipelineError::from)?
            .flatten();
        if source_run.as_ref().is_none_or(|run| {
            !run_record_matches_close_basis_context(
                run,
                &context.request.envelope.project_id,
                &context.request.task_id,
                context.request.change_unit_id.as_str(),
                context.current_scope_revision,
                Some(context.request.baseline_ref.as_str()),
            )
        }) {
            validation_plan_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                "evidence_updates[].observation_refs",
                "evidence update observation refs must have current same-scope Run provenance",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
        if require_strong_reuse {
            let source_kind: EvidenceSourceKind = parse_owner_storage_value(
                "evidence_observations",
                record.evidence_observation_id.clone(),
                "source_kind",
                &record.source_kind,
            )?;
            let assurance_level: EvidenceAssuranceLevel = parse_owner_storage_value(
                "evidence_observations",
                record.evidence_observation_id.clone(),
                "assurance_level",
                &record.assurance_level,
            )?;
            if evidence_provenance_class(source_kind, assurance_level)
                != EvidenceProvenanceClass::Strong
            {
                validation_plan_error(
                    context.request.envelope.dry_run,
                    Some(context.project_state.state_version),
                    "evidence_updates[].observation_refs",
                    "supported evidence may only reuse target-matching observations with sufficient provenance",
                )?;
                unreachable!("validation_plan_error always returns Err");
            }
        }
    }
    Ok(())
}

fn reused_observation_inputs_for_update(
    context: &RecordRunObservationContext<'_>,
    update: &EvidenceCoverageUpdate,
) -> Result<Vec<EvidenceObservationInput>, PlanError> {
    let mut inputs = Vec::with_capacity(update.observation_refs.len());
    for observation_ref in &update.observation_refs {
        let record = context
            .store
            .evidence_observation_record(observation_ref.record_id.as_str())
            .map_err(CorePipelineError::from)?
            .expect("validated reused observation exists");
        let assurance_level = parse_owner_storage_value(
            "evidence_observations",
            record.evidence_observation_id.clone(),
            "assurance_level",
            &record.assurance_level,
        )?;
        inputs.push(EvidenceObservationInput {
            target: update.target.clone(),
            source_kind: EvidenceSourceKind::ReusedEvidence,
            assurance_level,
            observed_by_actor_source: None.into(),
            tool_name: None.into(),
            tool_invocation_id: None.into(),
            tool_metadata: JsonObject::new(),
            input_refs: vec![state_ref(
                StateRecordKind::EvidenceObservation,
                &record.evidence_observation_id,
                &context.request.envelope.project_id,
                Some(&context.request.task_id),
                Some(context.project_state.state_version),
            )],
            source_refs: Vec::new(),
            output_artifact_refs: update.supporting_artifact_refs.clone(),
            limitations: vec![
                "Reuses target-matching observation provenance from the current scope.".to_owned(),
            ],
            observed_at: context.now.clone(),
        });
    }
    Ok(inputs)
}

fn evidence_observation_record_matches_target(
    record: &EvidenceObservationRecord,
    target: &EvidenceTarget,
) -> bool {
    match target {
        EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id,
        } => {
            record.acceptance_criterion_id.as_deref() == Some(acceptance_criterion_id.as_str())
                && record.evidence_claim_id.is_none()
        }
        EvidenceTarget::SupplementalClaim {
            evidence_claim_id, ..
        } => {
            record.evidence_claim_id.as_deref() == Some(evidence_claim_id.as_str())
                && record.acceptance_criterion_id.is_none()
        }
    }
}

fn validate_supporting_run_refs(
    context: &RecordRunObservationContext<'_>,
    refs: &[StateRecordRef],
) -> Result<(), PlanError> {
    for record_ref in refs {
        let is_current_run = record_ref.record_id == context.run_ref.record_id;
        let stored_run = if is_current_run {
            None
        } else {
            context
                .store
                .run_record(record_ref.record_id.as_str())
                .map_err(CorePipelineError::from)?
        };
        if record_ref.record_kind != StateRecordKind::Run
            || record_ref.project_id != context.request.envelope.project_id
            || record_ref.task_id.as_ref() != Some(&context.request.task_id)
            || record_ref.record_id.as_str().trim().is_empty()
            || (!is_current_run
                && stored_run.as_ref().is_none_or(|run| {
                    run.task_id != context.request.task_id.as_str()
                        || run.project_id != context.request.envelope.project_id.as_str()
                        || run.status != "recorded"
                }))
        {
            validation_plan_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                "evidence_updates[].supporting_run_refs",
                "supporting_run_refs must identify existing Runs for the request Task",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
    }
    Ok(())
}

fn validate_evidence_gap_refs(
    context: &RecordRunObservationContext<'_>,
    refs: &[StateRecordRef],
) -> Result<(), PlanError> {
    let active = context
        .store
        .active_blocker_refs(
            &context.request.task_id,
            context.project_state.state_version,
        )
        .map_err(CorePipelineError::from)?;
    for record_ref in refs {
        if record_ref.record_kind != StateRecordKind::Blocker
            || record_ref.project_id != context.request.envelope.project_id
            || record_ref.task_id.as_ref() != Some(&context.request.task_id)
            || !active
                .iter()
                .any(|stored| stored.record_id == record_ref.record_id.as_str())
        {
            validation_plan_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                "evidence_updates[].gap_refs",
                "gap_refs must identify active blockers for the request Task",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
    }
    Ok(())
}

fn canonical_evidence_artifact_refs(
    context: &RecordRunObservationContext<'_>,
    field: &'static str,
    refs: &[ArtifactRef],
) -> Result<Vec<ArtifactRef>, PlanError> {
    let mut canonical = BTreeMap::new();
    for artifact_ref in refs {
        let newly_registered = context
            .registered_artifacts
            .iter()
            .find(|registered| registered.artifact_id == artifact_ref.artifact_id);
        if artifact_ref.project_id != context.request.envelope.project_id
            || artifact_ref.task_id != context.request.task_id
        {
            validation_plan_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                field,
                "evidence artifact refs must identify existing artifacts owned by the request project and Task",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
        let canonical_ref = if let Some(registered) = newly_registered {
            registered.clone()
        } else {
            let stored = context
                .store
                .artifact_record(artifact_ref.artifact_id.as_str())
                .map_err(CorePipelineError::from)?;
            let owner_link = context
                .store
                .artifact_has_task_owner_link(
                    artifact_ref.artifact_id.as_str(),
                    context.request.task_id.as_str(),
                )
                .map_err(CorePipelineError::from)?;
            let Some(stored) = stored else {
                validation_plan_error(
                    context.request.envelope.dry_run,
                    Some(context.project_state.state_version),
                    field,
                    "evidence artifact refs must identify existing artifacts owned by the request project and Task",
                )?;
                unreachable!("validation_plan_error always returns Err");
            };
            if stored.project_id != context.request.envelope.project_id.as_str()
                || stored.task_id != context.request.task_id.as_str()
                || !owner_link
            {
                validation_plan_error(
                    context.request.envelope.dry_run,
                    Some(context.project_state.state_version),
                    field,
                    "evidence artifact refs must identify existing artifacts owned by the request project and Task",
                )?;
                unreachable!("validation_plan_error always returns Err");
            }
            artifact_ref_from_verified_record(
                context.store,
                &stored,
                None,
                Some(context.planned_state_version),
            )?
        };
        canonical
            .entry(canonical_ref.artifact_id.as_str().to_owned())
            .or_insert(canonical_ref);
    }
    Ok(canonical.into_values().collect())
}

fn output_artifact_refs_for_observation(
    context: &RecordRunObservationContext<'_>,
    input: &EvidenceObservationInput,
) -> Vec<ArtifactRef> {
    input
        .output_artifact_refs
        .iter()
        .cloned()
        .chain(
            context
                .artifact_plans
                .iter()
                .filter(|plan| plan.evidence_target.as_ref() == Some(&input.target))
                .map(|plan| plan.artifact_ref.clone()),
        )
        .chain(
            context
                .registered_artifacts
                .iter()
                .filter(|artifact| {
                    input.output_artifact_refs.iter().any(|existing| {
                        existing.artifact_id == artifact.artifact_id
                            && existing.project_id == artifact.project_id
                    })
                })
                .cloned(),
        )
        .collect()
}

fn observation_refs_by_target(
    plans: &[RecordRunObservationPlan],
) -> BTreeMap<EvidenceTarget, Vec<StateRecordRef>> {
    let mut refs_by_target: BTreeMap<EvidenceTarget, Vec<StateRecordRef>> = BTreeMap::new();
    for plan in plans {
        refs_by_target
            .entry(plan.observation.target.clone())
            .or_default()
            .push(plan.observation_ref.clone());
    }
    refs_by_target
}

struct RecordRunCloseBasisContext<'a> {
    service: &'a CoreService,
    store: &'a CoreProjectStore,
    project_state: &'a ProjectStateHeader,
    request: &'a RecordRunRequest,
    task: &'a TaskRecord,
    run_ref: &'a StateRecordRef,
    write_ticket_scope: Option<&'a (WriteTicketRecord, WriteTicketAttemptScope)>,
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
        write_ticket_scope,
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
        write_ticket_scope,
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
    write_ticket_scope: Option<&(WriteTicketRecord, WriteTicketAttemptScope)>,
) -> Result<Vec<SensitiveActionRequirement>, PlanError> {
    let mut requirements =
        previous_current_sensitive_action_requirements(store, project_state, request, task)?;
    if let Some((record, scope)) = write_ticket_scope {
        if let Some(requirement) =
            sensitive_action_requirement_from_write_ticket(store, run_ref, record, scope)?
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

fn sensitive_action_requirement_from_write_ticket(
    store: &CoreProjectStore,
    run_ref: &StateRecordRef,
    record: &WriteTicketRecord,
    scope: &WriteTicketAttemptScope,
) -> Result<Option<SensitiveActionRequirement>, PlanError> {
    let sensitive_categories = normalized_string_set(&scope.sensitive_categories);
    if sensitive_categories.is_empty() {
        return Ok(None);
    }
    let action_kind = scope.intended_operation.trim().to_owned();
    if action_kind.is_empty() {
        return Err(PlanError::Core(CorePipelineError::Store(
            StoreError::corrupt_owner_state_json(
                "write_tickets",
                record.write_ticket_id.clone(),
                "attempt_scope_json",
            ),
        )));
    }
    let normalized_paths =
        normalize_product_paths(&store.project_record().repo_root, &scope.intended_paths).map_err(
            |_| {
                PlanError::Core(CorePipelineError::Store(
                    StoreError::corrupt_owner_state_json(
                        "write_tickets",
                        record.write_ticket_id.clone(),
                        "attempt_scope_json",
                    ),
                ))
            },
        )?;
    if normalized_paths.is_empty() {
        return Err(PlanError::Core(CorePipelineError::Store(
            StoreError::corrupt_owner_state_json(
                "write_tickets",
                record.write_ticket_id.clone(),
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
        source_write_ticket_ref: write_ticket_ref(
            record,
            run_ref
                .produced_at_state_version
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
        normalized.entry(key).or_insert(normalized_ref);
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
    if !run_record_matches_close_basis_context(
        record,
        &context.request.envelope.project_id,
        &context.request.task_id,
        context.request.change_unit_id.as_str(),
        context.current_scope_revision,
        Some(context.request.baseline_ref.as_str()),
    ) {
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

fn close_basis_ref_identity_key(record_ref: &StateRecordRef) -> (String, String, String) {
    state_record_ref_identity_key(record_ref)
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
    let verified_invocation = context.verified_invocation;
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
        verified_invocation,
        input,
        handle,
        &record,
        context.now,
    )?;

    let artifact_id = allocate_artifact_id(service, store).map_err(PlanError::Core)?;
    let uri = format!(
        "volicord-artifact://{}/{}",
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
        created_by_actor_source: Some(
            record
                .created_by_actor_source
                .parse::<ActorSource>()
                .map_err(|_| {
                    CorePipelineError::Store(StoreError::corrupt_owner_state_value(
                        "artifact_staging",
                        handle.handle_id.as_str(),
                        "created_by_actor_source",
                    ))
                })?,
        )
        .into(),
        storage_ref: Some(StorageRef::new(uri.clone())).into(),
    };
    let source_mutation = Some(CoreStorageMutation::PromoteStagedArtifact(
        ArtifactPromotion {
            handle_id: handle.handle_id.as_str().to_owned(),
            artifact_id: artifact_id.as_str().to_owned(),
            task_id: request.task_id.as_str().to_owned(),
            run_id: run_id.as_str().to_owned(),
            expected_created_by_actor_source: verified_invocation
                .actor_source
                .to_canonical_string(),
            expected_sha256: sha256,
            expected_size_bytes: size_bytes,
            expected_redaction_state: record.redaction_state.clone(),
            expected_expires_at: stored_expires_at.to_string(),
            uri,
            retention_json: "{}".to_owned(),
            producer_json: serde_json::to_string(&json!({
                "display_name": display_name,
                "content_type": content_type,
                "created_by_actor_source": verified_invocation.actor_source,
                "artifact_input_id": input.artifact_input_id.as_str(),
                "relation_hint": input.relation_hint,
                "evidence_target": input.evidence_target
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
        evidence_target: input.evidence_target.as_ref().cloned(),
        source_mutation,
        run_link,
    })
}

fn validate_staged_artifact_record(
    project_state: &ProjectStateHeader,
    request: &RecordRunRequest,
    verified_invocation: &VerifiedInvocationContext,
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
    let verified_actor_source = verified_invocation.actor_source.to_canonical_string();
    if record.created_by_actor_source != verified_actor_source
        || handle.created_by_actor_source.to_canonical_string() != record.created_by_actor_source
    {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_actor_source_mismatch",
            "staged artifact provenance does not match the verified actor source",
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
        evidence_target: input.evidence_target.as_ref().cloned(),
        source_mutation: None,
        run_link,
    })
}

fn validate_write_ticket_for_run(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &RecordRunRequest,
    record: &WriteTicketRecord,
    observed_changes: &ObservedChanges,
    now: DateTime<Utc>,
) -> Result<WriteTicketAttemptScope, PlanError> {
    if record.status == "consumed" || record.status == "revoked" {
        let reason = if record.status == "consumed" {
            "consumed"
        } else {
            "revoked"
        };
        return Err(PlanError::Response(Box::new(
            write_ticket_invalid_response(
                &request.envelope,
                Some(project_state.state_version),
                reason,
                "write ticket is not active",
            ),
        )));
    }
    if record.basis_state_version != project_state.state_version {
        return Err(PlanError::Response(Box::new(
            stale_write_ticket_basis_response(
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
            write_ticket_invalid_response(
                &request.envelope,
                Some(project_state.state_version),
                reason,
                "write ticket is not active",
            ),
        )));
    }
    if write_ticket_is_expired(record, now).map_err(CorePipelineError::from)? {
        return Err(PlanError::Response(Box::new(
            write_ticket_invalid_response(
                &request.envelope,
                Some(project_state.state_version),
                "expired",
                "write ticket is expired",
            ),
        )));
    }
    let scope: WriteTicketAttemptScope = decode_required_json::<PersistedWriteTicketAttemptScope>(
        "write_tickets",
        record.write_ticket_id.clone(),
        "attempt_scope_json",
        Some(&record.attempt_scope_json),
    )?
    .into();
    let scope_paths =
        normalize_product_paths(&store.project_record().repo_root, &scope.intended_paths).map_err(
            |_| {
                PlanError::Core(CorePipelineError::Store(
                    StoreError::corrupt_owner_state_json(
                        "write_tickets",
                        record.write_ticket_id.clone(),
                        "attempt_scope_json",
                    ),
                ))
            },
        )?;
    if let Some(mismatch) = run_write_ticket_mismatch(
        record,
        &scope,
        &request.task_id,
        &request.change_unit_id,
        &request.baseline_ref,
        observed_changes,
        &scope_paths,
    ) {
        return write_ticket_mismatch(request, project_state, mismatch.reason, mismatch.message);
    }
    Ok(scope)
}

fn write_ticket_mismatch(
    request: &RecordRunRequest,
    project_state: &ProjectStateHeader,
    reason: &'static str,
    message: &'static str,
) -> Result<WriteTicketAttemptScope, PlanError> {
    Err(PlanError::Response(Box::new(
        write_ticket_invalid_response(
            &request.envelope,
            Some(project_state.state_version),
            reason,
            message,
        ),
    )))
}

fn build_record_run_evidence_summary(
    context: &RecordRunObservationContext<'_>,
    request: &RecordRunRequest,
    run_ref: &StateRecordRef,
    registered_artifacts: &[ArtifactRef],
    artifact_plans: &[RecordRunArtifactPlan],
    observation_refs_by_target: &BTreeMap<EvidenceTarget, Vec<StateRecordRef>>,
) -> Result<Option<volicord_types::EvidenceSummary>, PlanError> {
    if request.evidence_updates.is_empty() {
        return Ok(None);
    }
    let mut coverage_items = Vec::new();
    for update in &request.evidence_updates {
        let mut item = EvidenceCoverageItem {
            target: update.target.clone(),
            coverage_state: update.coverage_state.into(),
            supporting_run_refs: update.supporting_run_refs.clone(),
            observation_refs: update.observation_refs.clone(),
            supporting_artifact_refs: canonical_evidence_artifact_refs(
                context,
                "evidence_updates[].supporting_artifact_refs",
                &update.supporting_artifact_refs,
            )?,
            gap_refs: update.gap_refs.clone(),
        };
        if !item.supporting_run_refs.iter().any(|record_ref| {
            state_record_ref_identity_key(record_ref) == state_record_ref_identity_key(run_ref)
        }) {
            item.supporting_run_refs.push(run_ref.clone());
        }
        for plan in artifact_plans {
            if plan.evidence_target.as_ref() == Some(&item.target)
                && !item
                    .supporting_artifact_refs
                    .iter()
                    .any(|artifact| artifact.artifact_id == plan.artifact_ref.artifact_id)
            {
                item.supporting_artifact_refs
                    .push(plan.artifact_ref.clone());
            }
        }
        if let Some(observation_refs) = observation_refs_by_target.get(&item.target) {
            for observation_ref in observation_refs {
                if !item.observation_refs.iter().any(|existing| {
                    state_record_ref_identity_key(existing)
                        == state_record_ref_identity_key(observation_ref)
                }) {
                    item.observation_refs.push(observation_ref.clone());
                }
            }
        }
        if item.coverage_state == EvidenceCoverageState::Supported
            && item.supporting_artifact_refs.iter().any(|artifact_ref| {
                artifact_ref.availability != ArtifactAvailability::Available
                    || artifact_ref.integrity_status != ArtifactIntegrityStatus::Verified
            })
        {
            item.coverage_state = EvidenceCoverageState::Stale;
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
    let observation_refs = unique_state_record_refs(
        coverage_items
            .iter()
            .flat_map(|item| item.observation_refs.clone())
            .collect(),
    );
    let status = evidence_status_for_items(&coverage_items);
    Ok(Some(volicord_types::EvidenceSummary {
        evidence_state: Some(EvidenceDisplayState::Attached),
        status,
        coverage_items,
        artifact_refs,
        observation_refs,
        updated_by_run_ref: Some(run_ref.clone()),
    }))
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
        "evidence_target": input.evidence_target
    }))?)
}
