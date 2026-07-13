use super::*;
use volicord_types::OperationCategory;

#[derive(Debug, Clone)]
struct ReconciliationPlan {
    task_id: TaskId,
    storage_mutations: Vec<CoreStorageMutation>,
    event_payload: JsonObject,
    result_fields: JsonObject,
    dry_run_summary: DryRunSummary,
}

#[derive(Debug, Clone)]
struct PlannedResolution {
    record: UnrecordedChangeRecord,
    basis: UnrecordedChangeResolutionBasis,
    resolved_by_actor_source: ActorSource,
    capture_basis: String,
    user_action_resolution_ref: Option<StateRecordRef>,
    resolved_at: UtcTimestamp,
}

#[derive(Debug, Clone)]
struct PlannedUserAction {
    unrecorded_change_ref: StateRecordRef,
    candidate: UserActionDraft,
    user_action: Option<UserActionRequest>,
    mutation: Option<CoreStorageMutation>,
}

#[derive(Debug, Clone)]
pub(super) struct ResolutionCandidate {
    basis: UnrecordedChangeResolutionBasis,
    actor_source: ActorSource,
    capture_basis: String,
    user_action_resolution_ref: Option<StateRecordRef>,
}

impl CoreService {
    /// Executes `volicord.reconcile_changes` for unrecorded-change findings.
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

        let policy_operation_category =
            reconcile_policy_operation_category(invocation.operation_category);
        let prepared = match prepare_or_response(
            self,
            MethodName::ReconcileChanges,
            request.envelope.clone(),
            request_json,
            invocation,
            MethodPolicy::exact(
                policy_operation_category,
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
        let now = prepared.operation_now.clone();
        if !request.envelope.dry_run {
            if let Err(error) = session_watch::run_session_watch_check(
                &prepared.store,
                &prepared.context.verified_invocation,
                Some(&request.task_id),
                &now,
            ) {
                return core_error_response(&request.envelope, Some(state_version), error);
            }
        }
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
                    dry_run_summary: plan.dry_run_summary,
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

fn reconcile_policy_operation_category(
    invocation_operation_category: OperationCategory,
) -> OperationCategory {
    match invocation_operation_category {
        OperationCategory::AgentWorkflow | OperationCategory::LocalRecovery => {
            invocation_operation_category
        }
        _ => OperationCategory::AgentWorkflow,
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
    let resolved_authorities = resolved_user_action_authorities_for_all_kinds(
        store,
        project_state,
        &request.envelope,
        &request.task_id,
        now,
    )?;
    let existing_pending_authorities = pending_user_action_authorities_for_plan(
        store,
        project_state,
        &request.envelope,
        &request.task_id,
        now,
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
    let write_tickets = store
        .write_tickets_for_task(&request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
    let user_channel_guard_health =
        adjusted_guard_health(store, verified_invocation, &request, &[])?;
    let can_resolve_in_chat =
        close_task::user_channel_can_resolve_in_chat(user_channel_guard_health.as_ref());

    let mut planned_resolutions = Vec::new();
    let mut planned_user_actions = Vec::new();
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
                        .user_action_resolution_id
                        .as_ref()
                        .expect("validated accepted_by_user request has a resolution id"),
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
                    user_action_resolution_ref: authority.user_action_resolution_id.as_ref().map(
                        |resolution_id| {
                            state_ref(
                                StateRecordKind::UserActionResolution,
                                resolution_id,
                                &request.envelope.project_id,
                                Some(&request.task_id),
                                Some(project_state.state_version),
                            )
                        },
                    ),
                    resolved_at: now.clone(),
                });
                continue;
            }
        }

        if let Some(candidate) = deterministic_resolution(
            store,
            record,
            &request.task_id,
            &runs,
            &write_tickets,
            project_state.state_version,
            *now.as_datetime(),
        )?
        .or_else(|| {
            accepted_resolution_candidate(&unrecorded_ref, &resolved_authorities, &request.task_id)
        }) {
            planned_resolutions.push(PlannedResolution {
                record: record.clone(),
                basis: candidate.basis,
                resolved_by_actor_source: candidate.actor_source,
                capture_basis: candidate.capture_basis,
                user_action_resolution_ref: candidate.user_action_resolution_ref,
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
            let user_action_plan = plan_reconciliation_user_action(
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
                !request.envelope.dry_run,
            )?;
            planned_user_actions.push(user_action_plan);
        }
        unresolved_findings.push(unrecorded_finding(
            record,
            &request,
            project_state.state_version,
            can_resolve_in_chat,
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
        planned_user_actions
            .iter()
            .filter_map(|user_action| user_action.mutation.clone()),
    );

    let planned_state_version = if storage_mutations.is_empty() || request.envelope.dry_run {
        project_state.state_version
    } else {
        project_state.state_version + 1
    };
    for finding in &mut unresolved_findings {
        normalize_next_action_collection(
            std::slice::from_mut(&mut finding.next_action),
            planned_state_version,
        );
    }
    let projected_pending_refs = projected_pending_refs(
        store,
        project_state,
        &request,
        planned_state_version,
        &planned_user_actions,
        now,
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
    let write_ticket_summary = projected_write_ticket_summary(
        store,
        &request.task_id,
        planned_state_version,
        *now.as_datetime(),
        Some(guarantee_display.clone()),
    )?;
    let current_close_basis = projected_close_basis(store, &request.task_id)?;
    let evidence_summary = projected_evidence_summary(
        store,
        &request.envelope.project_id,
        planned_state_version,
        &task,
    )?
    .map(|summary| evidence_summary_for_display(summary, current_close_basis.as_ref()));
    let mut pending_authorities = existing_pending_authorities.clone();
    pending_authorities.extend(
        planned_user_actions
            .iter()
            .filter_map(|user_action| user_action.user_action.as_ref())
            .map(user_action_authority_from_state),
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
        acceptance_criteria: active_acceptance_criteria_for_task(store, &request.task_id)?,
        pending_user_action_refs: projected_pending_refs.clone(),
        blocker_refs,
        write_ticket_summary,
        evidence_summary,
        evidence_gate: Some(close_plan.evidence_gate),
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
    let mut result_next_actions = reconcile_next_actions(
        &unresolved_findings,
        &planned_user_actions,
        user_channel_guard_health.as_ref(),
        request.envelope.dry_run,
    );
    normalize_next_action_collection(&mut result_next_actions, planned_state_version);
    let summary_card = summary_card_for_core(SummaryCardBuild {
        task: Some(&task),
        recording: if storage_mutations.is_empty() {
            "read_only"
        } else {
            "core_committed"
        },
        profile: profile_summary_text(
            close_plan.guard_health.as_ref(),
            state.guarantee_display.as_ref(),
        ),
        write_ticket: write_ticket_summary_text(true, state.write_ticket_summary.as_ref()),
        evidence: evidence_gate_summary_text(true, state.evidence_gate.as_ref()),
        pending_user_actions: projected_pending_refs.len(),
        changes: changes_summary_text(true, unresolved_findings.len() as u64),
        close_status: close_state_text(close_plan.close_state).to_owned(),
        verified_invocation,
        next_action: primary_next_action(&result_next_actions, &close_plan.blockers),
    });
    let result = ReconcileChangesResult {
        base: placeholder_base(),
        summary_card,
        task_ref,
        unresolved_changes: unresolved_findings.clone(),
        resolved_changes,
        pending_user_action_summaries: agent_safe_pending_user_action_summaries(
            projected_pending_refs,
        ),
        rejected_resolution_requests,
        state,
        close_blockers: close_plan.blockers.clone(),
        guard_health: close_plan.guard_health.clone(),
        next_actions: result_next_actions.clone(),
    };
    let event_payload = object_from_value(json!({
        "task_id": request.task_id,
        "resolved_unrecorded_change_ids": planned_resolutions
            .iter()
            .map(|resolution| resolution.record.unrecorded_change_id.clone())
            .collect::<Vec<_>>(),
        "requested_user_action_request_ids": planned_user_actions
            .iter()
            .filter_map(|user_action| user_action.user_action.as_ref())
            .map(|action| action.user_action_request_id.as_str().to_owned())
            .collect::<Vec<_>>(),
        "rejected_resolution_count": result.rejected_resolution_requests.len()
    }))?;
    let dry_run_summary = dry_run_summary_for_reconciliation(
        &planned_resolutions,
        &planned_user_actions,
        &unresolved_findings,
        &close_plan.blockers,
        result_next_actions,
    )?;

    Ok(ReconciliationPlan {
        task_id: request.task_id,
        storage_mutations,
        event_payload,
        result_fields: strip_base(serde_json::to_value(result)?)?,
        dry_run_summary,
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
    resolved_authorities: &[UserActionAuthority],
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
    let Some(user_action_resolution_id) = request_item.user_action_resolution_id.as_ref() else {
        return Ok(Some(volicord_types::UnrecordedChangeRejection {
            unrecorded_change_id: request_item.unrecorded_change_id.clone(),
            basis: request_item.basis,
            code: "missing_user_action_resolution".to_owned(),
            message:
                "accepted_by_user requires an immutable user-action resolution linked to the finding"
                    .to_owned(),
        }));
    };
    if accepted_authority_for_request(
        user_action_resolution_id,
        unrecorded_ref,
        resolved_authorities,
        &request.task_id,
    )
    .is_none()
    {
        return Ok(Some(volicord_types::UnrecordedChangeRejection {
            unrecorded_change_id: UnrecordedChangeId::new(record.unrecorded_change_id.clone()),
            basis: request_item.basis,
            code: "user_action_resolution_not_accepted".to_owned(),
            message: "the supplied resolution is absent, not accepted by the local user, stale, or not linked to this finding".to_owned(),
        }));
    }
    Ok(None)
}

fn deterministic_resolution(
    store: &CoreProjectStore,
    record: &UnrecordedChangeRecord,
    task_id: &TaskId,
    runs: &[RunObservedChangesRecord],
    write_tickets: &[WriteTicketRecord],
    state_version: u64,
    now: DateTime<Utc>,
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
    if let Some(candidate) = session_watch::watcher_reverted_resolution(store, record)? {
        return Ok(Some(candidate));
    }
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
    if let Some(candidate) =
        session_watch::watcher_expected_write_resolution(store, record, task_id)?
    {
        return Ok(Some(candidate));
    }
    let mut active_matches = Vec::new();
    for write_ticket in write_tickets {
        let attempt_scope: WriteTicketAttemptScope = decode_required_json(
            "write_tickets",
            write_ticket.write_ticket_id.clone(),
            "attempt_scope_json",
            Some(&write_ticket.attempt_scope_json),
        )?;
        if attempt_scope.product_file_write_intended
            && paths_are_authorized(&observed_paths, &attempt_scope.intended_paths)
        {
            if write_ticket.status == "consumed" && write_ticket.consumed_by_run_id.is_some() {
                return Ok(Some(system_resolution(
                    UnrecordedChangeResolutionBasis::CoveredByWriteTicket,
                    "core_deterministic_write_ticket",
                )));
            }
            if write_ticket.status == "active"
                && write_ticket.basis_state_version == state_version
                && !write_ticket_is_expired(write_ticket, now)?
            {
                active_matches.push(write_ticket.write_ticket_id.clone());
            }
        }
    }
    if active_matches.len() == 1 {
        return Ok(Some(system_resolution(
            UnrecordedChangeResolutionBasis::CoveredByWriteTicket,
            "core_deterministic_write_ticket",
        )));
    }
    Ok(None)
}

pub(super) fn system_resolution(
    basis: UnrecordedChangeResolutionBasis,
    capture_basis: &str,
) -> ResolutionCandidate {
    ResolutionCandidate {
        basis,
        actor_source: ActorSource::System,
        capture_basis: capture_basis.to_owned(),
        user_action_resolution_ref: None,
    }
}

fn accepted_resolution_candidate(
    unrecorded_ref: &StateRecordRef,
    resolved_authorities: &[UserActionAuthority],
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
            user_action_resolution_ref: authority.user_action_resolution_id.as_ref().map(
                |resolution_id| {
                    state_ref(
                        StateRecordKind::UserActionResolution,
                        resolution_id,
                        &unrecorded_ref.project_id,
                        Some(task_id),
                        unrecorded_ref.produced_at_state_version.as_ref().copied(),
                    )
                },
            ),
        })
}

fn accepted_authority_for_request<'a>(
    user_action_resolution_id: &UserActionResolutionId,
    unrecorded_ref: &StateRecordRef,
    resolved_authorities: &'a [UserActionAuthority],
    task_id: &TaskId,
) -> Option<&'a UserActionAuthority> {
    resolved_authorities.iter().find(|authority| {
        authority.user_action_resolution_id.as_deref() == Some(user_action_resolution_id.as_str())
            && accepted_authority_for_unrecorded(authority, unrecorded_ref, task_id)
    })
}

fn accepted_authority_for_unrecorded(
    authority: &UserActionAuthority,
    unrecorded_ref: &StateRecordRef,
    task_id: &TaskId,
) -> bool {
    user_action_has_current_basis(authority)
        && authority.status == UserActionStatus::Resolved
        && authority.action_kind == UserActionKind::ProductDecision
        && authority.task_id == *task_id
        && authority.machine_action == Some(UserActionOptionAction::Accept)
        && authority.resolution_outcome == Some(JudgmentResolutionOutcome::Accepted)
        && authority.resolved_by_actor_source == Some(ActorSource::LocalUser)
        && verified_user_channel_provenance(authority)
        && authority
            .affected_refs
            .iter()
            .any(|affected| same_state_record(affected, unrecorded_ref))
        && authority.resolution.as_ref().is_some_and(|resolution| {
            matches!(
                resolution,
                UserActionResolutionBody::Choice {
                    machine_action: UserActionOptionAction::Accept,
                    resolution_outcome: JudgmentResolutionOutcome::Accepted,
                    ..
                }
            )
        })
}

fn pending_authority_for_unrecorded<'a>(
    unrecorded_ref: &StateRecordRef,
    pending_authorities: &'a [UserActionAuthority],
    task_id: &TaskId,
) -> Option<&'a UserActionAuthority> {
    pending_authorities.iter().find(|authority| {
        authority.status == UserActionStatus::Pending
            && authority.action_kind == UserActionKind::ProductDecision
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

pub(super) fn observed_paths(record: &UnrecordedChangeRecord) -> Result<Vec<String>, ()> {
    let paths = serde_json::from_str::<Vec<String>>(&record.observed_paths_json).map_err(|_| ())?;
    if paths.iter().any(|path| path.trim().is_empty()) {
        return Err(());
    }
    Ok(paths)
}

#[allow(clippy::too_many_arguments)]
fn plan_reconciliation_user_action(
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
    materialize_mutation: bool,
) -> Result<PlannedUserAction, PlanError> {
    let option_input = UserActionOptionInput {
        option_id: UserActionOptionId::new("accept"),
        label: "Accept observed change".to_owned(),
        description:
            "Record that the user accepts this observed Product Repository change as intentional."
                .to_owned(),
        consequence:
            "The linked unrecorded-change finding can be resolved with basis accepted_by_user."
                .to_owned(),
        is_default: true,
    };
    let context = UserActionContext {
        summary: record.summary.clone(),
        related_refs: vec![unrecorded_ref.clone()],
        artifact_refs: Vec::new(),
        visible_risks: Vec::new(),
        constraints: vec![
            "This accepts only the linked unrecorded Product Repository change.".to_owned(),
            "This is not evidence, test sufficiency, review completion, final acceptance, or residual-risk acceptance.".to_owned(),
        ],
    };
    let question =
        "Do you accept this observed Product Repository change as intentional for this Task?"
            .to_owned();
    let candidate = UserActionDraft::Choice(Box::new(UserActionChoiceDraft {
        judgment_kind: JudgmentKind::ProductDecision,
        presentation: JudgmentPresentation::Short,
        question: question.clone(),
        options: Some(vec![option_input]).into(),
        context: context.clone(),
        affected_refs: vec![unrecorded_ref.clone()],
        sensitive_action_scope: RequiredNullable::null(),
    }));
    let (user_action, mutation) = if materialize_mutation {
        let coordinate_change_unit_id =
            current_change_unit.map(|record| ChangeUnitId::new(record.change_unit_id.clone()));
        let internal_request = RequestUserActionRequest {
            envelope: request.envelope.clone(),
            task_id: request.task_id.clone(),
            change_unit_id: coordinate_change_unit_id.clone().into(),
            action: candidate.clone(),
            required_for: vec![UserActionRequiredFor::Informational],
            expires_at: RequiredNullable::null(),
        };
        internal_request.action.validate_bounds().map_err(|error| {
            PlanError::Response(Box::new(
                validation_rejected(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    error.field(),
                    error.message(),
                )
                .expect("internal user-action validation response should serialize"),
            ))
        })?;
        user_action::validate_required_for_compatibility(
            internal_request.action.action_kind(),
            &internal_request.required_for,
            request.envelope.dry_run,
            project_state.state_version,
        )?;
        let coordinates =
            reconciliation_user_action_coordinates(task, current_change_unit, project_state)?;
        let (body, basis) = user_action::canonical_request_body_and_basis(
            store,
            project_state,
            &internal_request,
            coordinates,
        )?;
        body.validate_bounds().map_err(|error| {
            PlanError::Response(Box::new(
                validation_rejected(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    error.field(),
                    error.message(),
                )
                .expect("internal user-action validation response should serialize"),
            ))
        })?;
        let materialized = user_action::materialize_user_action_request(
            user_action::MaterializeUserActionRequestInput {
                service,
                store,
                project_state,
                verified_invocation,
                envelope: &request.envelope,
                source_method: MethodName::ReconcileChanges,
                task_id: &request.task_id,
                coordinate_change_unit_id,
                body,
                basis,
                required_for: vec![UserActionRequiredFor::Informational],
                expires_at: RequiredNullable::null(),
                created_at: now.clone(),
                metadata_json: serde_json::to_string(&json!({
                "created_by": "volicord.reconcile_changes",
                "unrecorded_change_id": record.unrecorded_change_id
                }))?,
            },
        )?;
        (
            Some(materialized.public_request),
            Some(materialized.mutation),
        )
    } else {
        (None, None)
    };
    Ok(PlannedUserAction {
        unrecorded_change_ref: unrecorded_ref.clone(),
        candidate,
        user_action,
        mutation,
    })
}

fn reconciliation_user_action_coordinates(
    task: &TaskRecord,
    current_change_unit: Option<&ChangeUnitRecord>,
    project_state: &ProjectStateHeader,
) -> Result<UserActionBasisCoordinates, PlanError> {
    let scope = StoredScope::from_task(task).map_err(PlanError::Core)?;
    Ok(UserActionBasisCoordinates {
        task_id: TaskId::new(task.task_id.clone()),
        change_unit_id: current_change_unit
            .map(|record| ChangeUnitId::new(record.change_unit_id.clone()))
            .into(),
        scope_revision: task.scope_revision,
        baseline_ref: scope.baseline_ref.map(BaselineRef::new).into(),
        created_at_state_version: project_state.state_version,
        compatibility_status: UserActionBasisStatus::Current,
    })
}

fn projected_pending_refs(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &ReconcileChangesRequest,
    planned_state_version: u64,
    planned_user_actions: &[PlannedUserAction],
    now: &UtcTimestamp,
) -> Result<Vec<StateRecordRef>, PlanError> {
    let mut refs = store
        .pending_user_action_refs(&request.task_id, planned_state_version, now)
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
    refs.extend(
        planned_user_actions
            .iter()
            .filter_map(|user_action| user_action.user_action.as_ref())
            .map(|user_action| {
                state_ref(
                    StateRecordKind::UserActionRequest,
                    user_action.user_action_request_id.as_str(),
                    &request.envelope.project_id,
                    Some(&request.task_id),
                    Some(planned_state_version),
                )
            }),
    );
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
    pending_authorities: Vec<UserActionAuthority>,
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
            utc_timestamp(now),
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
        summary.local_web_consent_available = verified_invocation.local_web_consent_available;
        session_watch::apply_session_watch_status(store, verified_invocation, summary)?;
        let resolved_for_connection = planned_resolutions
            .iter()
            .filter(|resolution| resolution.record.connection_internal_id == connection_id.as_str())
            .count() as u64;
        summary.unresolved_unrecorded_change_count = summary
            .unresolved_unrecorded_change_count
            .saturating_sub(resolved_for_connection);
        close_task::refresh_control_surface(summary);
    }
    Ok(guard_health)
}

fn resolution_mutation(resolution: &PlannedResolution) -> CoreResult<CoreStorageMutation> {
    Ok(CoreStorageMutation::ResolveUnrecordedChange(
        UnrecordedChangeResolutionUpdate {
            unrecorded_change_id: resolution.record.unrecorded_change_id.clone(),
            resolution_json: serde_json::to_string(&json!({
                "resolution_basis": resolution.basis,
                "capture_basis": resolution.capture_basis,
                "user_action_resolution_ref": resolution.user_action_resolution_ref,
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
    can_resolve_in_chat: bool,
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
        can_resolve_in_chat,
        next_action: NextActionSummary {
            presentation_role: NextActionPresentationRole::Primary,
            action_kind: NextActionKind::ReconcileChanges,
            owner_method: Some(MethodName::ReconcileChanges),
            allowed_operation_categories: vec![
                OperationCategory::AgentWorkflow,
                OperationCategory::LocalRecovery,
            ],
            label: "Run reconciliation; the user must resolve any created action through a User Channel."
                .to_owned(),
            blocking_question: None,
            expected_state_version: RequiredNullable::null(),
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
        user_action_resolution_ref: resolution.user_action_resolution_ref.clone().into(),
        resolved_at: resolution.resolved_at.clone(),
    }
}

fn reconcile_next_actions(
    unresolved_findings: &[UnrecordedChangeFinding],
    planned_user_actions: &[PlannedUserAction],
    guard_health: Option<&GuardHealthSummary>,
    dry_run: bool,
) -> Vec<NextActionSummary> {
    if planned_user_actions.is_empty() && unresolved_findings.is_empty() {
        return Vec::new();
    }
    if !planned_user_actions.is_empty() {
        if dry_run {
            return vec![NextActionSummary {
                presentation_role: NextActionPresentationRole::Primary,
                action_kind: NextActionKind::ReconcileChanges,
                owner_method: Some(MethodName::ReconcileChanges),
                allowed_operation_categories: vec![
                    OperationCategory::AgentWorkflow,
                    OperationCategory::LocalRecovery,
                ],
                label: "Run reconciliation without dry-run to create pending user-action requests."
                    .to_owned(),
                blocking_question: None,
                expected_state_version: RequiredNullable::null(),
                required_refs: planned_user_actions
                    .iter()
                    .map(|user_action| user_action.unrecorded_change_ref.clone())
                    .collect(),
            }];
        }
        return vec![NextActionSummary {
            presentation_role: NextActionPresentationRole::Primary,
            action_kind: NextActionKind::ResolveUserAction,
            owner_method: Some(MethodName::ResolveUserAction),
            allowed_operation_categories: vec![OperationCategory::UserOnly],
            label: close_task::user_channel_pending_action_instruction(guard_health),
            blocking_question: None,
            expected_state_version: RequiredNullable::null(),
            required_refs: planned_user_actions
                .iter()
                .map(|user_action| user_action.unrecorded_change_ref.clone())
                .collect(),
        }];
    }
    vec![NextActionSummary {
        presentation_role: NextActionPresentationRole::Primary,
        action_kind: NextActionKind::ReconcileChanges,
        owner_method: Some(MethodName::ReconcileChanges),
        allowed_operation_categories: vec![
            OperationCategory::AgentWorkflow,
            OperationCategory::LocalRecovery,
        ],
        label: "Run reconciliation again after the user actions are resolved.".to_owned(),
        blocking_question: None,
        expected_state_version: RequiredNullable::null(),
        required_refs: unresolved_findings
            .iter()
            .map(|finding| finding.unrecorded_change_ref.clone())
            .collect(),
    }]
}

fn dry_run_summary_for_reconciliation(
    planned_resolutions: &[PlannedResolution],
    planned_user_actions: &[PlannedUserAction],
    unresolved_findings: &[UnrecordedChangeFinding],
    close_blockers: &[CloseReadinessBlocker],
    next_actions: Vec<NextActionSummary>,
) -> CoreResult<DryRunSummary> {
    let planned_effects = planned_effects_for_reconciliation(
        planned_resolutions,
        planned_user_actions,
        unresolved_findings,
        close_blockers,
    );
    let would_blockers = close_blockers
        .iter()
        .map(close_blocker_as_planned_blocker)
        .collect::<CoreResult<Vec<_>>>()?;
    let automatically_reconcilable = planned_resolutions
        .iter()
        .filter(|resolution| resolution.basis != UnrecordedChangeResolutionBasis::AcceptedByUser)
        .count();
    let accepted_user_resolutions = planned_resolutions
        .iter()
        .filter(|resolution| resolution.basis == UnrecordedChangeResolutionBasis::AcceptedByUser)
        .count();
    let needing_user_action = unresolved_findings.len();
    let mut diagnostics = vec![
        format!("automatically_reconcilable_changes={automatically_reconcilable}"),
        format!("accepted_user_resolution_changes={accepted_user_resolutions}"),
        format!("changes_needing_user_action={needing_user_action}"),
        format!("would_create_user_actions={}", planned_user_actions.len()),
        close_blocker_diagnostic(planned_resolutions, unresolved_findings, close_blockers),
        "non_guarantees=no actor proof; no intent proof; no correctness proof".to_owned(),
    ];
    diagnostics.extend(planned_user_actions.iter().map(|user_action| {
        format!(
            "would_create_user_action_for={}; kind={}",
            user_action.unrecorded_change_ref.record_id.as_str(),
            storage_value(user_action.candidate.action_kind())
                .unwrap_or_else(|_| "unknown".to_owned())
        )
    }));
    Ok(DryRunSummary {
        planned_effects,
        would_blockers,
        would_errors: Vec::new(),
        next_actions,
        diagnostics,
    })
}

fn planned_effects_for_reconciliation(
    planned_resolutions: &[PlannedResolution],
    planned_user_actions: &[PlannedUserAction],
    unresolved_findings: &[UnrecordedChangeFinding],
    close_blockers: &[CloseReadinessBlocker],
) -> Vec<PlannedEffect> {
    let mut effects = Vec::new();
    let automatically_reconcilable = planned_resolutions
        .iter()
        .filter(|resolution| resolution.basis != UnrecordedChangeResolutionBasis::AcceptedByUser)
        .count();
    effects.push(PlannedEffect {
        target_kind: "reconciliation".to_owned(),
        action: "classify".to_owned(),
        description: format!(
            "Classify {automatically_reconcilable} automatically reconcilable change(s) and {} change(s) needing a user action.",
            unresolved_findings.len()
        ),
    });
    if !planned_resolutions.is_empty() {
        effects.push(PlannedEffect {
            target_kind: "unrecorded_change".to_owned(),
            action: "would_resolve".to_owned(),
            description: format!(
                "Would resolve {} unrecorded-change finding(s).",
                planned_resolutions.len()
            ),
        });
    }
    if !planned_user_actions.is_empty() {
        effects.push(PlannedEffect {
            target_kind: "user_action_request".to_owned(),
            action: "would_request".to_owned(),
            description: format!(
                "Would create {} pending user-action request(s).",
                planned_user_actions.len()
            ),
        });
    }
    let unresolved_blocker_remains = close_blockers
        .iter()
        .any(|blocker| blocker.code == "unresolved_unrecorded_changes");
    let close_action = if unresolved_blocker_remains {
        "would_remain_blocked"
    } else if !planned_resolutions.is_empty() {
        "would_reduce_blockers"
    } else {
        "would_remain_unchanged"
    };
    effects.push(PlannedEffect {
        target_kind: "close_readiness".to_owned(),
        action: close_action.to_owned(),
        description: close_blocker_diagnostic(
            planned_resolutions,
            unresolved_findings,
            close_blockers,
        ),
    });
    effects
}

fn close_blocker_diagnostic(
    planned_resolutions: &[PlannedResolution],
    unresolved_findings: &[UnrecordedChangeFinding],
    close_blockers: &[CloseReadinessBlocker],
) -> String {
    let unresolved_blocker_remains = close_blockers
        .iter()
        .any(|blocker| blocker.code == "unresolved_unrecorded_changes");
    if unresolved_blocker_remains {
        return "close_blockers=unresolved_unrecorded_changes would remain".to_owned();
    }
    if !planned_resolutions.is_empty() {
        return "close_blockers=unresolved_unrecorded_changes would be reduced".to_owned();
    }
    if !unresolved_findings.is_empty() {
        return "close_blockers=unresolved_unrecorded_changes would remain".to_owned();
    }
    "close_blockers=no unresolved_unrecorded_changes blocker projected".to_owned()
}

fn close_blocker_as_planned_blocker(blocker: &CloseReadinessBlocker) -> CoreResult<PlannedBlocker> {
    Ok(PlannedBlocker {
        source_kind: PlannedBlockerSourceKind::CloseReadiness,
        category: storage_value(blocker.category)?,
        code: blocker.code.clone(),
        message: blocker.message.clone(),
        related_refs: blocker.related_refs.clone(),
    })
}
