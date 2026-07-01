use super::*;

#[derive(Debug, Clone)]
struct ReconciliationPlan {
    task_id: TaskId,
    storage_mutations: Vec<CoreStorageMutation>,
    event_payload: JsonObject,
    result_fields: JsonObject,
    next_actions: Vec<NextActionSummary>,
    planned_effects: Vec<PlannedEffect>,
}

#[derive(Debug, Clone)]
struct PlannedResolution {
    record: UnrecordedChangeRecord,
    basis: UnrecordedChangeResolutionBasis,
    resolved_by_actor_source: ActorSource,
    capture_basis: String,
    user_judgment_ref: Option<StateRecordRef>,
    resolved_at: UtcTimestamp,
}

#[derive(Debug, Clone)]
struct PlannedJudgment {
    user_judgment: UserJudgment,
    mutation: CoreStorageMutation,
}

#[derive(Debug, Clone)]
struct ResolutionCandidate {
    basis: UnrecordedChangeResolutionBasis,
    actor_source: ActorSource,
    capture_basis: String,
    user_judgment_ref: Option<StateRecordRef>,
}

impl CoreService {
    /// Executes `volicord.reconcile_changes` for guarded unrecorded-change findings.
    pub fn reconcile_changes(
        &self,
        request: ReconcileChangesRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        if let Some(envelope_task_id) = request.envelope.task_id.as_ref() {
            if envelope_task_id != &request.task_id {
                return validation_rejected(
                    request.envelope.dry_run,
                    None,
                    "task_id",
                    "envelope.task_id must match ReconcileChangesRequest.task_id",
                );
            }
        } else {
            return validation_rejected(
                request.envelope.dry_run,
                None,
                "envelope.task_id",
                "reconcile_changes requires envelope.task_id to identify the Task",
            );
        }

        let prepared = match prepare_or_response(
            self,
            MethodName::ReconcileChanges,
            request.envelope.clone(),
            request_json,
            invocation,
            MethodPolicy::exact(
                request.operation_category(),
                TaskRequirement::Exact(request.task_id.clone()),
                ReplayPolicy::Committed,
                FreshnessPolicy::IfPresent,
                MethodEffectPolicy::CoreMutation,
            ),
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let state_version = prepared.context.project_state.state_version;
        let now = utc_timestamp(self.now());
        let plan = match plan_reconcile_changes(
            self,
            &prepared.store,
            &prepared.context.project_state,
            &prepared.context.verified_invocation,
            request.clone(),
            &now,
        ) {
            Ok(plan) => plan,
            Err(PlanError::Response(response)) => return Ok(*response),
            Err(PlanError::Core(error)) => {
                return core_error_response(&request.envelope, Some(state_version), error)
            }
        };

        if request.envelope.dry_run {
            return self.execute_prepared_request(
                prepared,
                OwnerPipelineBranch::DryRunPreview {
                    dry_run_summary: DryRunSummary {
                        planned_effects: plan.planned_effects,
                        would_blockers: Vec::new(),
                        would_errors: Vec::new(),
                        next_actions: plan.next_actions,
                        diagnostics: Vec::new(),
                    },
                },
            );
        }

        if plan.storage_mutations.is_empty() {
            return self.execute_prepared_request(
                prepared,
                OwnerPipelineBranch::ReadOnly {
                    result_fields: plan.result_fields,
                },
            );
        }

        self.execute_prepared_request(
            prepared,
            OwnerPipelineBranch::CommitMutation {
                result_fields: plan.result_fields,
                event_kind: "unrecorded_changes_reconciled".to_owned(),
                event_payload: plan.event_payload,
                task_id: Some(plan.task_id),
                change_unit_id: None,
                storage_mutations: plan.storage_mutations,
            },
        )
    }
}

fn plan_reconcile_changes(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: &VerifiedInvocationContext,
    request: ReconcileChangesRequest,
    now: &UtcTimestamp,
) -> Result<ReconciliationPlan, PlanError> {
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
    let unresolved = unresolved_records_for_request(store, verified_invocation, &request)?;
    let request_by_change = resolution_requests_by_change(&request.resolution_requests);
    let resolved_authorities = resolved_judgment_authorities_for_all_kinds(
        store,
        project_state,
        &request.envelope,
        &request.task_id,
    )?;
    let existing_pending_authorities = pending_judgment_authorities_for_plan(
        store,
        project_state,
        &request.envelope,
        &request.task_id,
    )?;
    let runs = store
        .run_observed_changes_for_task(&request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
    let write_checks = store
        .write_checks_for_task(&request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;

    let mut planned_resolutions = Vec::new();
    let mut planned_judgments = Vec::new();
    let mut unresolved_findings = Vec::new();
    let mut rejected_resolution_requests = Vec::new();
    let mut seen_change_ids = BTreeSet::new();
    for record in &unresolved {
        seen_change_ids.insert(record.unrecorded_change_id.clone());
        let unrecorded_ref = unrecorded_change_ref(record, &request, project_state.state_version);
        let requested = request_by_change.get(record.unrecorded_change_id.as_str());
        if let Some(requested) = requested {
            if let Some(rejection) = validate_requested_resolution(
                requested,
                record,
                &unrecorded_ref,
                &resolved_authorities,
                &request,
            )? {
                rejected_resolution_requests.push(rejection);
            } else if requested.basis == UnrecordedChangeResolutionBasis::AcceptedByUser {
                let authority = accepted_authority_for_request(
                    requested
                        .user_judgment_id
                        .as_ref()
                        .expect("validated accepted_by_user request has a judgment id"),
                    &unrecorded_ref,
                    &resolved_authorities,
                    &request.task_id,
                )
                .expect("validated accepted_by_user request has accepted authority");
                planned_resolutions.push(PlannedResolution {
                    record: record.clone(),
                    basis: UnrecordedChangeResolutionBasis::AcceptedByUser,
                    resolved_by_actor_source: ActorSource::LocalUser,
                    capture_basis: authority
                        .resolved_verification_basis
                        .clone()
                        .unwrap_or_else(|| "user_channel".to_owned()),
                    user_judgment_ref: Some(state_ref(
                        StateRecordKind::UserJudgment,
                        &authority.judgment_id,
                        &request.envelope.project_id,
                        Some(&request.task_id),
                        Some(project_state.state_version),
                    )),
                    resolved_at: now.clone(),
                });
                continue;
            }
        }

        if let Some(candidate) =
            deterministic_resolution(record, &runs, &write_checks)?.or_else(|| {
                accepted_resolution_candidate(
                    &unrecorded_ref,
                    &resolved_authorities,
                    &request.task_id,
                )
            })
        {
            planned_resolutions.push(PlannedResolution {
                record: record.clone(),
                basis: candidate.basis,
                resolved_by_actor_source: candidate.actor_source,
                capture_basis: candidate.capture_basis,
                user_judgment_ref: candidate.user_judgment_ref,
                resolved_at: now.clone(),
            });
            continue;
        }

        if pending_authority_for_unrecorded(
            &unrecorded_ref,
            &existing_pending_authorities,
            &request.task_id,
        )
        .is_none()
        {
            let judgment = plan_reconciliation_judgment(
                service,
                store,
                project_state,
                verified_invocation,
                &request,
                &task,
                current_change_unit.as_ref(),
                record,
                &unrecorded_ref,
                now,
            )?;
            planned_judgments.push(judgment);
        }
        unresolved_findings.push(unrecorded_finding(
            record,
            &request,
            project_state.state_version,
        )?);
    }

    for request_item in &request.resolution_requests {
        if !seen_change_ids.contains(request_item.unrecorded_change_id.as_str()) {
            rejected_resolution_requests.push(volicord_types::UnrecordedChangeRejection {
                unrecorded_change_id: request_item.unrecorded_change_id.clone(),
                basis: request_item.basis,
                code: "not_unresolved_for_task".to_owned(),
                message: "resolution request does not identify an unresolved finding for this Task"
                    .to_owned(),
            });
        }
    }

    let mut storage_mutations = planned_resolutions
        .iter()
        .map(resolution_mutation)
        .collect::<CoreResult<Vec<_>>>()
        .map_err(PlanError::Core)?;
    storage_mutations.extend(
        planned_judgments
            .iter()
            .map(|judgment| judgment.mutation.clone()),
    );

    let planned_state_version = if storage_mutations.is_empty() || request.envelope.dry_run {
        project_state.state_version
    } else {
        project_state.state_version + 1
    };
    let projected_pending_refs = projected_pending_refs(
        store,
        project_state,
        &request,
        planned_state_version,
        &planned_judgments,
    )?;
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
    let guarantee_display =
        guarantee_display_for_invocation(store, verified_invocation, planned_state_version)?;
    let write_check_summary = projected_write_check_summary(
        store,
        &request.task_id,
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
    let mut pending_authorities = existing_pending_authorities.clone();
    pending_authorities.extend(
        planned_judgments
            .iter()
            .map(|judgment| user_judgment_authority_from_state(&judgment.user_judgment, None)),
    );
    let close_plan = projected_close_check_with_guard_health(
        store,
        project_state,
        verified_invocation,
        &request,
        &task,
        current_change_unit.clone(),
        projected_pending_refs.clone(),
        blocker_refs.clone(),
        evidence_summary.clone(),
        pending_authorities,
        &planned_resolutions,
        *now.as_datetime(),
        planned_state_version,
    )?;
    let state = build_state_summary(SummaryBuild {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &task,
        current_change_unit: current_change_unit.as_ref(),
        pending_user_judgment_refs: projected_pending_refs.clone(),
        blocker_refs,
        write_check_summary,
        evidence_summary,
        close_state: Some(close_plan.close_state),
        close_blockers: close_plan.blockers.clone(),
        guard_health: close_plan.guard_health.clone(),
        guarantee_display: Some(guarantee_display),
    })?;
    let task_ref = state_ref(
        StateRecordKind::Task,
        request.task_id.as_str(),
        &request.envelope.project_id,
        Some(&request.task_id),
        Some(planned_state_version),
    );
    let resolved_changes = planned_resolutions
        .iter()
        .map(|resolution| resolution_summary(resolution, &request, planned_state_version))
        .collect::<Vec<_>>();
    let result_next_actions =
        reconcile_next_actions(&request, &unresolved_findings, &planned_judgments);
    let result = ReconcileChangesResult {
        base: placeholder_base(),
        task_ref,
        unresolved_changes: unresolved_findings,
        resolved_changes,
        pending_user_judgment_refs: projected_pending_refs,
        rejected_resolution_requests,
        state,
        close_blockers: close_plan.blockers,
        guard_health: close_plan.guard_health,
        next_actions: result_next_actions.clone(),
    };
    let event_payload = object_from_value(json!({
        "task_id": request.task_id,
        "resolved_unrecorded_change_ids": planned_resolutions
            .iter()
            .map(|resolution| resolution.record.unrecorded_change_id.clone())
            .collect::<Vec<_>>(),
        "requested_user_judgment_ids": planned_judgments
            .iter()
            .map(|judgment| judgment.user_judgment.judgment_id.as_str().to_owned())
            .collect::<Vec<_>>(),
        "rejected_resolution_count": result.rejected_resolution_requests.len()
    }))?;
    let planned_effects =
        planned_effects_for_reconciliation(&planned_resolutions, &planned_judgments);

    Ok(ReconciliationPlan {
        task_id: request.task_id,
        storage_mutations,
        event_payload,
        result_fields: strip_base(serde_json::to_value(result)?)?,
        next_actions: result_next_actions,
        planned_effects,
    })
}

fn unresolved_records_for_request(
    store: &CoreProjectStore,
    verified_invocation: &VerifiedInvocationContext,
    request: &ReconcileChangesRequest,
) -> Result<Vec<UnrecordedChangeRecord>, PlanError> {
    let connection_id = verified_invocation.actor_source.agent_connection_id();
    let records = volicord_store::guards::list_unresolved_unrecorded_changes(
        store.runtime_home(),
        request.envelope.project_id.as_str(),
        connection_id.as_ref().map(|id| id.as_str()),
    )
    .map_err(CorePipelineError::from)
    .map_err(PlanError::Core)?;
    Ok(records
        .into_iter()
        .filter(|record| {
            record.task_id.as_deref().is_none()
                || record.task_id.as_deref() == Some(request.task_id.as_str())
        })
        .collect())
}

fn resolution_requests_by_change(
    requests: &[UnrecordedChangeResolutionRequest],
) -> BTreeMap<&str, &UnrecordedChangeResolutionRequest> {
    let mut by_change = BTreeMap::new();
    for request in requests {
        by_change
            .entry(request.unrecorded_change_id.as_str())
            .or_insert(request);
    }
    by_change
}

fn validate_requested_resolution(
    request_item: &UnrecordedChangeResolutionRequest,
    record: &UnrecordedChangeRecord,
    unrecorded_ref: &StateRecordRef,
    resolved_authorities: &[JudgmentAuthority],
    request: &ReconcileChangesRequest,
) -> Result<Option<volicord_types::UnrecordedChangeRejection>, PlanError> {
    if request_item.basis != UnrecordedChangeResolutionBasis::AcceptedByUser {
        return Ok(Some(volicord_types::UnrecordedChangeRejection {
            unrecorded_change_id: request_item.unrecorded_change_id.clone(),
            basis: request_item.basis,
            code: "system_resolution_basis_not_caller_owned".to_owned(),
            message:
                "this resolution basis must be verified by Core, not supplied as an agent dismissal"
                    .to_owned(),
        }));
    }
    let Some(user_judgment_id) = request_item.user_judgment_id.as_ref() else {
        return Ok(Some(volicord_types::UnrecordedChangeRejection {
            unrecorded_change_id: request_item.unrecorded_change_id.clone(),
            basis: request_item.basis,
            code: "missing_user_judgment".to_owned(),
            message:
                "accepted_by_user requires a resolved user-owned judgment linked to the finding"
                    .to_owned(),
        }));
    };
    if accepted_authority_for_request(
        user_judgment_id,
        unrecorded_ref,
        resolved_authorities,
        &request.task_id,
    )
    .is_none()
    {
        return Ok(Some(volicord_types::UnrecordedChangeRejection {
            unrecorded_change_id: UnrecordedChangeId::new(record.unrecorded_change_id.clone()),
            basis: request_item.basis,
            code: "user_judgment_not_accepted".to_owned(),
            message: "the supplied judgment is absent, unresolved, not accepted by the local user, stale, or not linked to this finding".to_owned(),
        }));
    }
    Ok(None)
}

fn deterministic_resolution(
    record: &UnrecordedChangeRecord,
    runs: &[RunObservedChangesRecord],
    write_checks: &[WriteCheckRecord],
) -> CoreResult<Option<ResolutionCandidate>> {
    let observed_paths = match observed_paths(record) {
        Ok(paths) => paths,
        Err(()) => {
            return Ok(Some(system_resolution(
                UnrecordedChangeResolutionBasis::InvalidObservation,
                "core_deterministic_invalid_observation",
            )))
        }
    };
    if observed_paths.is_empty() {
        return Ok(Some(system_resolution(
            UnrecordedChangeResolutionBasis::NotProductChange,
            "core_deterministic_not_product_change",
        )));
    }
    if runs.iter().any(|run| {
        run.status == "recorded"
            && run.observed_changes.product_file_write_observed
            && paths_are_authorized(&observed_paths, &run.observed_changes.changed_paths)
    }) {
        return Ok(Some(system_resolution(
            UnrecordedChangeResolutionBasis::RecordedAsExpectedWrite,
            "core_deterministic_recorded_run",
        )));
    }
    for write_check in write_checks {
        if write_check.status != "consumed" || write_check.consumed_by_run_id.is_none() {
            continue;
        }
        let attempt_scope: WriteCheckAttemptScope = decode_required_json(
            "write_checks",
            write_check.write_check_id.clone(),
            "attempt_scope_json",
            Some(&write_check.attempt_scope_json),
        )?;
        if attempt_scope.product_file_write_intended
            && paths_are_authorized(&observed_paths, &attempt_scope.intended_paths)
        {
            return Ok(Some(system_resolution(
                UnrecordedChangeResolutionBasis::CoveredByWriteReadiness,
                "core_deterministic_write_readiness",
            )));
        }
    }
    Ok(None)
}

fn system_resolution(
    basis: UnrecordedChangeResolutionBasis,
    capture_basis: &str,
) -> ResolutionCandidate {
    ResolutionCandidate {
        basis,
        actor_source: ActorSource::System,
        capture_basis: capture_basis.to_owned(),
        user_judgment_ref: None,
    }
}

fn accepted_resolution_candidate(
    unrecorded_ref: &StateRecordRef,
    resolved_authorities: &[JudgmentAuthority],
    task_id: &TaskId,
) -> Option<ResolutionCandidate> {
    resolved_authorities
        .iter()
        .find(|authority| accepted_authority_for_unrecorded(authority, unrecorded_ref, task_id))
        .map(|authority| ResolutionCandidate {
            basis: UnrecordedChangeResolutionBasis::AcceptedByUser,
            actor_source: ActorSource::LocalUser,
            capture_basis: authority
                .resolved_verification_basis
                .clone()
                .unwrap_or_else(|| "user_channel".to_owned()),
            user_judgment_ref: Some(state_ref(
                StateRecordKind::UserJudgment,
                &authority.judgment_id,
                &unrecorded_ref.project_id,
                Some(task_id),
                unrecorded_ref.state_version.as_ref().copied(),
            )),
        })
}

fn accepted_authority_for_request<'a>(
    user_judgment_id: &UserJudgmentId,
    unrecorded_ref: &StateRecordRef,
    resolved_authorities: &'a [JudgmentAuthority],
    task_id: &TaskId,
) -> Option<&'a JudgmentAuthority> {
    resolved_authorities.iter().find(|authority| {
        authority.judgment_id == user_judgment_id.as_str()
            && accepted_authority_for_unrecorded(authority, unrecorded_ref, task_id)
    })
}

fn accepted_authority_for_unrecorded(
    authority: &JudgmentAuthority,
    unrecorded_ref: &StateRecordRef,
    task_id: &TaskId,
) -> bool {
    judgment_has_current_basis(authority)
        && authority.status == UserJudgmentStatus::Resolved
        && authority.judgment_kind == JudgmentKind::ProductDecision
        && authority.task_id == *task_id
        && authority.machine_action == Some(UserJudgmentOptionAction::Accept)
        && authority.resolution_outcome == Some(JudgmentResolutionOutcome::Accepted)
        && authority.resolved_by_actor_source == Some(ActorSource::LocalUser)
        && verified_user_channel_provenance(authority)
        && authority
            .affected_refs
            .iter()
            .any(|affected| same_state_record(affected, unrecorded_ref))
        && authority.resolution.as_ref().is_some_and(|resolution| {
            resolution.machine_action == UserJudgmentOptionAction::Accept
                && resolution.resolution_outcome == JudgmentResolutionOutcome::Accepted
                && resolution.resolved_by_actor_source == ActorSource::LocalUser
                && answer_branch_matches_kind(JudgmentKind::ProductDecision, &resolution.answer)
        })
}

fn pending_authority_for_unrecorded<'a>(
    unrecorded_ref: &StateRecordRef,
    pending_authorities: &'a [JudgmentAuthority],
    task_id: &TaskId,
) -> Option<&'a JudgmentAuthority> {
    pending_authorities.iter().find(|authority| {
        authority.status == UserJudgmentStatus::Pending
            && authority.judgment_kind == JudgmentKind::ProductDecision
            && authority.task_id == *task_id
            && authority
                .affected_refs
                .iter()
                .any(|affected| same_state_record(affected, unrecorded_ref))
    })
}

fn same_state_record(left: &StateRecordRef, right: &StateRecordRef) -> bool {
    left.record_kind == right.record_kind
        && left.record_id == right.record_id
        && left.project_id == right.project_id
}

fn observed_paths(record: &UnrecordedChangeRecord) -> Result<Vec<String>, ()> {
    let paths = serde_json::from_str::<Vec<String>>(&record.observed_paths_json).map_err(|_| ())?;
    if paths.iter().any(|path| path.trim().is_empty()) {
        return Err(());
    }
    Ok(paths)
}

fn plan_reconciliation_judgment(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: &VerifiedInvocationContext,
    request: &ReconcileChangesRequest,
    task: &TaskRecord,
    current_change_unit: Option<&ChangeUnitRecord>,
    record: &UnrecordedChangeRecord,
    unrecorded_ref: &StateRecordRef,
    now: &UtcTimestamp,
) -> Result<PlannedJudgment, PlanError> {
    let judgment_id = allocate_user_judgment_id(service, store).map_err(PlanError::Core)?;
    let options = vec![UserJudgmentOption {
        option_id: UserJudgmentOptionId::new("accept"),
        label: "Accept observed change".to_owned(),
        description:
            "Record that the user accepts this observed Product Repository change as intentional."
                .to_owned(),
        consequence:
            "The linked unrecorded-change finding can be resolved with basis accepted_by_user."
                .to_owned(),
        machine_action: UserJudgmentOptionAction::Accept,
        resolution_outcome: JudgmentResolutionOutcome::Accepted,
        is_default: true,
    }];
    let basis = reconciliation_judgment_basis(task, current_change_unit, project_state)?;
    let context = UserJudgmentContext {
        summary: record.summary.clone(),
        related_refs: vec![unrecorded_ref.clone()],
        artifact_refs: Vec::new(),
        visible_risks: Vec::new(),
        constraints: vec![
            "This accepts only the linked unrecorded Product Repository change.".to_owned(),
            "This is not evidence, test sufficiency, review completion, final acceptance, or residual-risk acceptance.".to_owned(),
        ],
    };
    let user_judgment = UserJudgment {
        judgment_id: judgment_id.clone(),
        project_id: request.envelope.project_id.clone(),
        task_id: request.task_id.clone(),
        change_unit_id: current_change_unit
            .map(|record| ChangeUnitId::new(record.change_unit_id.clone())),
        judgment_kind: JudgmentKind::ProductDecision,
        status: UserJudgmentStatus::Pending,
        presentation: JudgmentPresentation::Short,
        question:
            "Do you accept this observed Product Repository change as intentional for this Task?"
                .to_owned(),
        options: options.clone(),
        context: context.clone(),
        affected_refs: vec![unrecorded_ref.clone()],
        basis: basis.clone(),
        required_for: vec![JudgmentRequiredFor::Informational],
        resolution: None,
        expires_at: None,
        created_at: now.clone(),
        resolved_at: None,
    };
    let mutation = CoreStorageMutation::InsertUserJudgment(UserJudgmentInsert {
        judgment_id: judgment_id.as_str().to_owned(),
        task_id: request.task_id.as_str().to_owned(),
        change_unit_id: current_change_unit.map(|record| record.change_unit_id.clone()),
        judgment_kind: storage_value(JudgmentKind::ProductDecision).map_err(PlanError::Core)?,
        request_json: serde_json::to_string(&json!({
            "presentation": JudgmentPresentation::Short,
            "question": user_judgment.question.clone(),
            "required_for": [JudgmentRequiredFor::Informational],
            "expires_at": RequiredNullable::<UtcTimestamp>::null()
        }))?,
        context_json: serde_json::to_string(&context)?,
        options_json: serde_json::to_string(&PersistedUserJudgmentOptions::current(options))?,
        affected_refs_json: serde_json::to_string(&[unrecorded_ref])?,
        artifact_refs_json: "[]".to_owned(),
        sensitive_action_scope_json: "{}".to_owned(),
        basis_json: serde_json::to_string(&basis)?,
        basis_status: JudgmentBasisCompatibilityStatus::Current,
        requested_by_actor_source: verified_invocation.actor_source.to_canonical_string(),
        requested_at: now.to_string(),
        metadata_json: serde_json::to_string(&json!({
            "created_by": "volicord.reconcile_changes",
            "unrecorded_change_id": record.unrecorded_change_id
        }))?,
    });
    Ok(PlannedJudgment {
        user_judgment,
        mutation,
    })
}

fn reconciliation_judgment_basis(
    task: &TaskRecord,
    current_change_unit: Option<&ChangeUnitRecord>,
    project_state: &ProjectStateHeader,
) -> Result<JudgmentBasis, PlanError> {
    let scope = StoredScope::from_task(task).map_err(PlanError::Core)?;
    Ok(JudgmentBasis {
        task_id: TaskId::new(task.task_id.clone()),
        change_unit_id: current_change_unit
            .map(|record| ChangeUnitId::new(record.change_unit_id.clone()))
            .into(),
        scope_revision: task.scope_revision,
        close_basis_revision: Some(task.close_basis_revision).into(),
        baseline_ref: scope.baseline_ref.map(BaselineRef::new).into(),
        result_refs: Vec::new(),
        residual_risk_ids: Vec::new(),
        sensitive_action_scope: RequiredNullable::null(),
        created_at_state_version: project_state.state_version,
        compatibility_status: JudgmentBasisCompatibilityStatus::Current,
    })
}

fn projected_pending_refs(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &ReconcileChangesRequest,
    planned_state_version: u64,
    planned_judgments: &[PlannedJudgment],
) -> Result<Vec<StateRecordRef>, PlanError> {
    let mut refs = store
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
    refs.extend(planned_judgments.iter().map(|judgment| {
        state_ref(
            StateRecordKind::UserJudgment,
            judgment.user_judgment.judgment_id.as_str(),
            &request.envelope.project_id,
            Some(&request.task_id),
            Some(planned_state_version),
        )
    }));
    Ok(refs)
}

#[allow(clippy::too_many_arguments)]
fn projected_close_check_with_guard_health(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: &VerifiedInvocationContext,
    request: &ReconcileChangesRequest,
    task: &TaskRecord,
    current_change_unit: Option<ChangeUnitRecord>,
    pending_refs: Vec<StateRecordRef>,
    blocker_refs: Vec<StateRecordRef>,
    evidence_summary: Option<EvidenceSummary>,
    pending_authorities: Vec<JudgmentAuthority>,
    planned_resolutions: &[PlannedResolution],
    now: DateTime<Utc>,
    planned_state_version: u64,
) -> Result<CloseTaskPlan, PlanError> {
    let projected_project_state = project_state_projection(
        project_state,
        planned_state_version,
        project_state
            .active_task_id
            .clone()
            .or_else(|| Some(request.task_id.as_str().to_owned())),
    );
    let mut context = close_context_with_pending_authorities(
        close_context_from_projection(
            task.clone(),
            current_change_unit,
            projected_close_basis(store, &request.task_id)?,
            pending_refs,
            blocker_refs,
            evidence_summary,
        ),
        pending_authorities,
    );
    context.guard_health =
        adjusted_guard_health(store, verified_invocation, request, planned_resolutions)?;
    projected_close_check(
        store,
        &projected_project_state,
        verified_invocation,
        &request.envelope,
        &request.task_id,
        context,
        now,
    )
}

fn adjusted_guard_health(
    store: &CoreProjectStore,
    verified_invocation: &VerifiedInvocationContext,
    request: &ReconcileChangesRequest,
    planned_resolutions: &[PlannedResolution],
) -> Result<Option<GuardHealthSummary>, PlanError> {
    let Some(connection_id) = verified_invocation.actor_source.agent_connection_id() else {
        return Ok(None);
    };
    let record = volicord_store::guards::guard_health_record(
        store.runtime_home(),
        request.envelope.project_id.as_str(),
        connection_id.as_str(),
    )
    .map_err(CorePipelineError::from)
    .map_err(PlanError::Core)?;
    let mut guard_health = close_task::guard_health_summary_from_record(record)?;
    if let Some(summary) = guard_health.as_mut() {
        let resolved_for_connection = planned_resolutions
            .iter()
            .filter(|resolution| resolution.record.connection_internal_id == connection_id.as_str())
            .count() as u64;
        summary.unresolved_unrecorded_change_count = summary
            .unresolved_unrecorded_change_count
            .saturating_sub(resolved_for_connection);
    }
    Ok(guard_health)
}

fn resolution_mutation(resolution: &PlannedResolution) -> CoreResult<CoreStorageMutation> {
    Ok(CoreStorageMutation::ResolveUnrecordedChange(
        UnrecordedChangeResolutionUpdate {
            unrecorded_change_id: resolution.record.unrecorded_change_id.clone(),
            resolution_json: serde_json::to_string(&json!({
                "schema_version": 1,
                "resolution_basis": resolution.basis,
                "capture_basis": resolution.capture_basis,
                "user_judgment_ref": resolution.user_judgment_ref,
                "resolved_by_method": "volicord.reconcile_changes"
            }))?,
            resolved_at: resolution.resolved_at.to_string(),
            resolved_by_actor_source: resolution.resolved_by_actor_source.to_canonical_string(),
        },
    ))
}

fn unrecorded_finding(
    record: &UnrecordedChangeRecord,
    request: &ReconcileChangesRequest,
    state_version: u64,
) -> CoreResult<UnrecordedChangeFinding> {
    Ok(UnrecordedChangeFinding {
        unrecorded_change_ref: unrecorded_change_ref(record, request, state_version),
        status: UnrecordedChangeStatus::Unresolved,
        summary: record.summary.clone(),
        observed_paths: observed_paths(record).unwrap_or_default(),
        detected_at: parse_owner_storage_value(
            "unrecorded_changes",
            record.unrecorded_change_id.clone(),
            "detected_at",
            &record.detected_at,
        )?,
        can_resolve_in_chat: true,
        next_action: NextActionSummary {
            action_kind: NextActionKind::ReconcileChanges,
            owner_method: Some(MethodName::ReconcileChanges),
            label: "Run reconciliation and answer any created user-owned judgment.".to_owned(),
            blocking_question: Some(
                "Does the user accept this observed Product Repository change as intentional?"
                    .to_owned(),
            ),
            required_refs: vec![unrecorded_change_ref(record, request, state_version)],
        },
    })
}

fn unrecorded_change_ref(
    record: &UnrecordedChangeRecord,
    request: &ReconcileChangesRequest,
    state_version: u64,
) -> StateRecordRef {
    let ref_task_id = record
        .task_id
        .as_ref()
        .map(|task_id| TaskId::new(task_id.clone()))
        .unwrap_or_else(|| request.task_id.clone());
    state_ref(
        StateRecordKind::UnrecordedChange,
        &record.unrecorded_change_id,
        &request.envelope.project_id,
        Some(&ref_task_id),
        Some(state_version),
    )
}

fn resolution_summary(
    resolution: &PlannedResolution,
    request: &ReconcileChangesRequest,
    state_version: u64,
) -> UnrecordedChangeResolutionSummary {
    UnrecordedChangeResolutionSummary {
        unrecorded_change_ref: unrecorded_change_ref(&resolution.record, request, state_version),
        resolution_basis: resolution.basis,
        resolved_by_actor_source: resolution.resolved_by_actor_source.clone(),
        capture_basis: resolution.capture_basis.clone(),
        user_judgment_ref: resolution.user_judgment_ref.clone().into(),
        resolved_at: resolution.resolved_at.clone(),
    }
}

fn reconcile_next_actions(
    request: &ReconcileChangesRequest,
    unresolved_findings: &[UnrecordedChangeFinding],
    planned_judgments: &[PlannedJudgment],
) -> Vec<NextActionSummary> {
    if planned_judgments.is_empty() && unresolved_findings.is_empty() {
        return Vec::new();
    }
    if !planned_judgments.is_empty() {
        return vec![NextActionSummary {
            action_kind: NextActionKind::RecordUserJudgment,
            owner_method: Some(MethodName::RecordUserJudgment),
            label: "Answer the created user-owned judgment before closing the Task.".to_owned(),
            blocking_question: Some(
                "Does the user accept the observed Product Repository change as intentional?"
                    .to_owned(),
            ),
            required_refs: planned_judgments
                .iter()
                .map(|judgment| {
                    state_ref(
                        StateRecordKind::UserJudgment,
                        judgment.user_judgment.judgment_id.as_str(),
                        &request.envelope.project_id,
                        Some(&request.task_id),
                        None,
                    )
                })
                .collect(),
        }];
    }
    vec![NextActionSummary {
        action_kind: NextActionKind::ReconcileChanges,
        owner_method: Some(MethodName::ReconcileChanges),
        label: "Run reconciliation again after user-owned judgments are answered.".to_owned(),
        blocking_question: None,
        required_refs: unresolved_findings
            .iter()
            .map(|finding| finding.unrecorded_change_ref.clone())
            .collect(),
    }]
}

fn planned_effects_for_reconciliation(
    planned_resolutions: &[PlannedResolution],
    planned_judgments: &[PlannedJudgment],
) -> Vec<PlannedEffect> {
    let mut effects = Vec::new();
    if !planned_resolutions.is_empty() {
        effects.push(PlannedEffect {
            target_kind: "unrecorded_change".to_owned(),
            action: "resolve".to_owned(),
            description: format!(
                "Resolve {} unrecorded-change finding(s).",
                planned_resolutions.len()
            ),
        });
    }
    if !planned_judgments.is_empty() {
        effects.push(PlannedEffect {
            target_kind: "user_judgment".to_owned(),
            action: "request".to_owned(),
            description: format!(
                "Create {} pending user-owned judgment request(s).",
                planned_judgments.len()
            ),
        });
    }
    effects
}
