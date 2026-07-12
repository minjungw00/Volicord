use super::*;

const STATUS_CONTINUITY_RECORD_LIMIT: usize = 8;

impl CoreService {
    /// Executes `volicord.status` as a read-only Core result.
    pub fn status(
        &self,
        request: StatusRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        let prepared = match prepare_or_response(
            self,
            MethodName::Status,
            request.envelope.clone(),
            request_json,
            invocation,
            MethodPolicy::exact(
                request.operation_category(),
                TaskRequirement::Optional,
                ReplayPolicy::None,
                FreshnessPolicy::None,
                MethodEffectPolicy::ReadOnly,
            ),
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let state_version = prepared.context.project_state.state_version;

        let task = match status_task(
            &prepared.store,
            &prepared.context.project_state,
            request.envelope.task_id.as_ref(),
        ) {
            Ok(task) => task,
            Err(error) => {
                return core_error_response(&request.envelope, Some(state_version), error)
            }
        };
        let result_fields = match status_result_fields(
            &prepared.store,
            &request.envelope,
            &prepared.context.project_state,
            &prepared.context.verified_invocation,
            task.as_ref(),
            &request.include,
            self.now(),
        ) {
            Ok(result_fields) => result_fields,
            Err(error) => {
                return plan_error_response(
                    &request.envelope,
                    &prepared.context.project_state,
                    error,
                )
            }
        };

        match self
            .execute_prepared_request(prepared, OwnerPipelineBranch::ReadOnly { result_fields })
        {
            Ok(response) => Ok(response),
            Err(error) => core_error_response(&request.envelope, Some(state_version), error),
        }
    }
}

fn status_task(
    store: &CoreProjectStore,
    _project_state: &ProjectStateHeader,
    envelope_task_id: Option<&TaskId>,
) -> CoreResult<Option<TaskRecord>> {
    match envelope_task_id {
        Some(task_id) => store.task_record(task_id).map_err(CorePipelineError::from),
        None => store.active_task_record().map_err(CorePipelineError::from),
    }
}

fn status_result_fields(
    store: &CoreProjectStore,
    envelope: &ToolEnvelope,
    project_state: &ProjectStateHeader,
    verified_invocation: &VerifiedInvocationContext,
    task: Option<&TaskRecord>,
    include: &StatusInclude,
    now: DateTime<Utc>,
) -> Result<JsonObject, PlanError> {
    let state_version = project_state.state_version;
    let project_id = &envelope.project_id;
    let mut active_task = None;
    let mut pending_user_judgments = Vec::new();
    let mut pending_inbox_items = Vec::new();
    let mut user_channel_availability_summary = None;
    let mut blocker_refs = Vec::new();
    let mut write_ticket_summary = None;
    let mut evidence_summary = None;
    let mut evidence_gate = None;
    let mut close_state = None;
    let mut current_close_basis = None;
    let mut risk_acceptance_coverage = None;
    let mut close_blockers = None;
    let mut guard_health = None;
    let mut coverage_summary = None;
    let mut continuity_summary = None;
    let mut task_flow = None;
    let mut authority_receipt = None;
    let mut next_actions = Vec::new();
    let mut receipt_next_actions = Vec::new();
    let mut card_pending_user_judgment_count = 0usize;
    let guarantee_profile = if include.guarantees {
        Some(
            store
                .project_enforcement_profile()
                .map_err(CorePipelineError::from)?
                .profile,
        )
    } else {
        None
    };
    let guarantee_projection = guarantee_profile
        .as_ref()
        .map(|profile| guarantee_display_from_profile(profile, verified_invocation, state_version));

    if let Some(task) = task {
        let task_id = TaskId::new(task.task_id.clone());
        let current_change_unit = store
            .current_change_unit(&task_id)
            .map_err(CorePipelineError::from)?;
        let task_ref = state_ref(
            StateRecordKind::Task,
            &task.task_id,
            project_id,
            Some(&task_id),
            Some(state_version),
        );
        let change_unit_ref = current_change_unit.as_ref().map(|record| {
            state_ref(
                StateRecordKind::ChangeUnit,
                &record.change_unit_id,
                project_id,
                Some(&task_id),
                Some(state_version),
            )
        });
        let task_next_actions = if is_terminal_lifecycle(&task.lifecycle_phase) {
            Vec::new()
        } else {
            next_actions_for_state(
                parse_task_mode(&task.mode)?,
                &task_ref,
                change_unit_ref.as_ref(),
                state_version,
            )
        };
        let all_pending_user_judgments =
            projected_pending_user_judgment_refs(store, &task_id, state_version)?;
        card_pending_user_judgment_count = all_pending_user_judgments.len();
        if include.pending_user_judgments {
            pending_user_judgments = all_pending_user_judgments.clone();
        }
        blocker_refs = projected_blocker_refs(store, &task_id, state_version)?;
        let projected_write_ticket = if include.write_ticket {
            projected_write_ticket_summary(
                store,
                &task_id,
                state_version,
                now,
                guarantee_projection.clone(),
            )?
        } else {
            None
        };
        write_ticket_summary = projected_write_ticket.clone();
        let close_plan = close_task::plan_close_task(
            store,
            project_state,
            Some(verified_invocation),
            guarantee_profile.as_ref(),
            close_task::CloseTaskPlanRequest::check(CheckCloseRequest {
                envelope: ToolEnvelope {
                    task_id: Some(task_id.clone()).into(),
                    ..envelope.clone()
                },
                task_id: task_id.clone(),
            }),
            &utc_timestamp(now),
        )?;
        let lifecycle_phase = parse_lifecycle_phase(&task.lifecycle_phase)?;
        let terminal_close_state = match lifecycle_phase {
            TaskLifecyclePhase::Completed => Some(CloseState::Closed),
            TaskLifecyclePhase::Cancelled => Some(CloseState::Cancelled),
            TaskLifecyclePhase::Superseded => Some(CloseState::Superseded),
            TaskLifecyclePhase::Shaping
            | TaskLifecyclePhase::Ready
            | TaskLifecyclePhase::Executing
            | TaskLifecyclePhase::WaitingUser
            | TaskLifecyclePhase::Blocked => None,
        };
        let effective_close_state = terminal_close_state.unwrap_or(close_plan.close_state);
        let effective_close_blockers = if terminal_close_state.is_some() {
            Vec::new()
        } else {
            close_plan.blockers.clone()
        };
        let mut effective_close_actions = close_next_actions(&effective_close_blockers);
        if effective_close_state == CloseState::Ready {
            let mut required_refs = vec![task_ref.clone()];
            required_refs.extend(change_unit_ref.clone());
            effective_close_actions.push(close_next_action(
                "Complete the current Task.",
                required_refs,
            ));
        }
        evidence_gate = Some(close_plan.evidence_gate);
        current_close_basis = close_plan.current_close_basis.clone();
        receipt_next_actions.extend(effective_close_actions.clone());
        receipt_next_actions.extend(task_next_actions.clone());
        if include.close {
            close_state = Some(status_close_state(effective_close_state));
            risk_acceptance_coverage = Some(close_plan.risk_acceptance_coverage.clone());
            close_blockers = Some(effective_close_blockers.clone());
            guard_health = close_plan.guard_health.clone();
            coverage_summary = close_plan
                .guard_health
                .as_ref()
                .map(close_task::coverage_summary_from_guard_health);
            next_actions.extend(effective_close_actions.clone());
        }
        if include.task {
            next_actions.extend(task_next_actions.clone());
        }
        let projected_evidence = if include.task || include.evidence || include.close {
            projected_evidence_summary(store, project_id, state_version, task)?
                .map(|summary| evidence_summary_for_display(summary, current_close_basis.as_ref()))
        } else {
            None
        };
        if include.evidence {
            evidence_summary = projected_evidence.clone();
        }
        if include.pending_user_judgments {
            let user_channel = UserChannelContext {
                guard_health: close_plan.guard_health.as_ref(),
                host_elicitation_available: verified_invocation.host_elicitation_available,
                local_web_consent_available: verified_invocation.local_web_consent_available,
            };
            user_channel_availability_summary = Some(user_channel_availability(user_channel));
            pending_inbox_items = pending_judgment_inbox_items(
                store,
                project_state,
                envelope,
                &task_id,
                state_version,
                user_channel,
            )?;
        }
        if include.task {
            let state = build_state_summary(SummaryBuild {
                project_id,
                state_version,
                task,
                current_change_unit: current_change_unit.as_ref(),
                acceptance_criteria: active_acceptance_criteria_for_task(store, &task_id)?,
                pending_user_judgment_refs: all_pending_user_judgments,
                blocker_refs: blocker_refs.clone(),
                write_ticket_summary: projected_write_ticket,
                evidence_summary: projected_evidence.clone(),
                evidence_gate,
                close_state: include.close.then_some(effective_close_state),
                close_blockers: if include.close {
                    effective_close_blockers.clone()
                } else {
                    Vec::new()
                },
                guard_health: include
                    .close
                    .then(|| close_plan.guard_health.clone())
                    .flatten(),
                guarantee_display: guarantee_projection.clone(),
            })?;
            active_task = Some(status_state_summary_value(state, include)?);
        }
        let latest_run = store
            .run_observed_changes_for_task(&task_id)
            .map_err(CorePipelineError::from)?
            .into_iter()
            .find(|record| record.status == "recorded");
        let latest_run_ref = latest_run.as_ref().map(|record| {
            state_ref(
                StateRecordKind::Run,
                &record.run_id,
                project_id,
                Some(&task_id),
                Some(state_version),
            )
        });
        let product_file_write_observed = latest_run
            .as_ref()
            .is_some_and(|record| record.observed_changes.product_file_write_observed);
        authority_receipt = Some(AuthorityReceipt {
            project_id: project_id.clone(),
            state_version,
            task_ref,
            change_unit_ref,
            scope_revision: task.scope_revision,
            latest_run_ref,
            product_file_write_observed,
            evidence_gate: Some(close_plan.evidence_gate),
            close_state: status_close_state(effective_close_state),
            close_blockers: effective_close_blockers,
            next_actor: AuthorityNextActor::None,
            next_action: None,
        });
    }
    if include.continuity {
        continuity_summary = Some(projected_continuity_summary(
            store,
            state_version,
            STATUS_CONTINUITY_RECORD_LIMIT,
        )?);
        if let Some(task) = task {
            task_flow = Some(projected_task_flow(store, task, state_version)?);
        }
    }
    next_actions = unique_next_actions(next_actions);
    normalize_next_action_collection(&mut next_actions, state_version);
    receipt_next_actions = unique_next_actions(receipt_next_actions);
    normalize_next_action_collection(&mut receipt_next_actions, state_version);
    if let Some(receipt) = authority_receipt.as_mut() {
        let next_action = receipt_next_actions.first().cloned();
        receipt.next_actor = next_action
            .as_ref()
            .map(authority_next_actor)
            .unwrap_or(AuthorityNextActor::None);
        receipt.next_action = next_action;
    }

    let close_blockers_slice = close_blockers.as_deref().unwrap_or(&[]);
    let summary_card = summary_card_for_core(SummaryCardBuild {
        task,
        recording: "read_only",
        profile: profile_summary_text(guard_health.as_ref(), guarantee_projection.as_ref()),
        write_ticket: write_ticket_summary_text(
            include.write_ticket,
            write_ticket_summary.as_ref(),
        ),
        evidence: evidence_gate_summary_text(
            include.evidence || include.close,
            evidence_gate.as_ref(),
        ),
        pending_user_judgments: card_pending_user_judgment_count,
        changes: changes_summary_text(
            include.close,
            guard_health
                .as_ref()
                .map(|health| health.unresolved_unrecorded_change_count)
                .unwrap_or(0),
        ),
        close_status: close_state_summary_text(include.close, close_state),
        verified_invocation,
        next_action: primary_next_action(&next_actions, close_blockers_slice),
    });

    let result = volicord_types::StatusResult {
        base: placeholder_base(),
        summary_card,
        active_task: None,
        status_summary: status_summary_for(task, close_state, close_blockers.as_deref()),
        next_actions,
        pending_user_judgments,
        pending_judgment_inbox_items: pending_inbox_items,
        user_channel_availability: user_channel_availability_summary,
        blocker_refs,
        write_ticket_summary,
        evidence_summary: include.evidence.then(|| evidence_summary.into()),
        evidence_gate: (include.evidence || include.close).then(|| evidence_gate.into()),
        close_state,
        current_close_basis: include.close.then(|| current_close_basis.into()),
        risk_acceptance_coverage,
        close_blockers,
        guard_health: include.close.then_some(guard_health).flatten(),
        coverage_summary: include.close.then_some(coverage_summary).flatten(),
        guarantee_display: guarantee_projection.map(RequiredNullable::some),
        continuity_summary,
        task_flow,
        authority_receipt,
    };
    let mut result_fields = strip_base(serde_json::to_value(result)?)?;
    if let Some(active_task) = active_task {
        result_fields.insert("active_task".to_owned(), active_task);
    }
    Ok(result_fields)
}

fn authority_next_actor(action: &NextActionSummary) -> AuthorityNextActor {
    if action
        .allowed_operation_categories
        .contains(&OperationCategory::UserOnly)
    {
        AuthorityNextActor::User
    } else if action
        .allowed_operation_categories
        .contains(&OperationCategory::AgentWorkflow)
    {
        AuthorityNextActor::Agent
    } else {
        AuthorityNextActor::None
    }
}

fn projected_task_flow(
    store: &CoreProjectStore,
    selected: &TaskRecord,
    state_version: u64,
) -> Result<Vec<TaskFlowItem>, PlanError> {
    let records = store.task_records().map_err(CorePipelineError::from)?;
    let mut connected = BTreeSet::from([selected.task_id.clone()]);
    loop {
        let before = connected.len();
        for record in &records {
            if connected.contains(&record.task_id)
                || record
                    .predecessor_task_id
                    .as_ref()
                    .is_some_and(|predecessor| connected.contains(predecessor))
            {
                connected.insert(record.task_id.clone());
                if let Some(predecessor) = record.predecessor_task_id.as_ref() {
                    connected.insert(predecessor.clone());
                }
            }
        }
        if connected.len() == before {
            break;
        }
    }
    records
        .into_iter()
        .filter(|record| connected.contains(&record.task_id))
        .map(|record| {
            let task_id = TaskId::new(record.task_id.clone());
            Ok(TaskFlowItem {
                task_ref: state_ref(
                    StateRecordKind::Task,
                    &record.task_id,
                    &ProjectId::new(record.project_id.clone()),
                    Some(&task_id),
                    Some(state_version),
                ),
                predecessor_task_ref: record.predecessor_task_id.as_ref().map(|predecessor| {
                    let predecessor_id = TaskId::new(predecessor.clone());
                    state_ref(
                        StateRecordKind::Task,
                        predecessor,
                        &ProjectId::new(record.project_id.clone()),
                        Some(&predecessor_id),
                        Some(state_version),
                    )
                }),
                relation: record
                    .lineage_relation
                    .as_deref()
                    .map(parse_task_lineage_relation)
                    .transpose()?,
                mode: parse_task_mode(&record.mode)?,
                work_phase: parse_work_phase(&record.work_phase)?,
                lifecycle_phase: parse_lifecycle_phase(&record.lifecycle_phase)?,
            })
        })
        .collect()
}

fn status_summary_for(
    task: Option<&TaskRecord>,
    close_state: Option<StatusCloseState>,
    close_blockers: Option<&[CloseReadinessBlocker]>,
) -> String {
    if task.is_none() {
        return "No current Task is selected.".to_owned();
    }
    if let Some(first_blocker) = close_blockers.and_then(|blockers| blockers.first()) {
        return format!("Close readiness is blocked by {}.", first_blocker.code);
    }
    match close_state {
        Some(StatusCloseState::Ready) => {
            "Close readiness is ready for the current Task.".to_owned()
        }
        Some(StatusCloseState::Closed) => "The selected Task is closed.".to_owned(),
        Some(StatusCloseState::Cancelled) => "The selected Task is cancelled.".to_owned(),
        Some(StatusCloseState::Superseded) => "The selected Task is superseded.".to_owned(),
        Some(StatusCloseState::Blocked) => "Close readiness is blocked.".to_owned(),
        Some(StatusCloseState::None) => "No close-readiness state is available.".to_owned(),
        None => "Current Task state is available.".to_owned(),
    }
}

fn projected_continuity_summary(
    store: &CoreProjectStore,
    state_version: u64,
    limit: usize,
) -> Result<Vec<ProjectContinuitySummary>, PlanError> {
    store
        .active_project_continuity_records(limit)
        .map_err(CorePipelineError::from)?
        .iter()
        .map(|record| {
            project_continuity_summary_from_record(record, state_version).map_err(PlanError::Core)
        })
        .collect()
}

fn status_close_state(close_state: CloseState) -> StatusCloseState {
    match close_state {
        CloseState::Ready => StatusCloseState::Ready,
        CloseState::Blocked => StatusCloseState::Blocked,
        CloseState::Closed => StatusCloseState::Closed,
        CloseState::Cancelled => StatusCloseState::Cancelled,
        CloseState::Superseded => StatusCloseState::Superseded,
    }
}

fn close_next_actions(blockers: &[CloseReadinessBlocker]) -> Vec<NextActionSummary> {
    blockers
        .iter()
        .flat_map(|blocker| blocker.next_actions.clone())
        .collect()
}

pub(super) fn unique_next_actions(actions: Vec<NextActionSummary>) -> Vec<NextActionSummary> {
    let mut seen = BTreeSet::new();
    actions
        .into_iter()
        .filter_map(|mut action| {
            action.required_refs = unique_state_record_refs(action.required_refs);
            let mut required_ref_keys = action
                .required_refs
                .iter()
                .map(state_record_ref_identity_key)
                .collect::<Vec<_>>();
            required_ref_keys.sort();
            let key = serde_json::to_string(&(
                &action.action_kind,
                &action.owner_method,
                &action.allowed_operation_categories,
                &action.label,
                &action.blocking_question,
                required_ref_keys,
            ))
            .unwrap_or_default();
            seen.insert(key).then_some(action)
        })
        .collect()
}

fn status_state_summary_value(
    state: volicord_types::StateSummary,
    include: &StatusInclude,
) -> CoreResult<Value> {
    let mut value = serde_json::to_value(state)?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| CorePipelineError::InvalidDispatch {
            detail: "state summary must serialize to a JSON object".to_owned(),
        })?;
    if !include.write_ticket {
        object.remove("write_ticket_summary");
    }
    if !include.evidence {
        object.remove("evidence_summary");
    }
    if !include.evidence && !include.close {
        object.remove("evidence_gate");
    }
    if !include.close {
        object.remove("close_state");
        object.remove("close_blockers");
        object.remove("guard_health");
    }
    if !include.guarantees {
        object.remove("guarantee_display");
    }
    Ok(value)
}
