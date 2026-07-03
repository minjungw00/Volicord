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
    let mut close_state = None;
    let mut current_close_basis = None;
    let mut risk_acceptance_coverage = None;
    let mut close_blockers = None;
    let mut card_evidence_summary = None;
    let mut prepared_input_available = false;
    let mut guard_health = None;
    let mut coverage_summary = None;
    let mut continuity_summary = None;
    let mut next_actions = Vec::new();
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
        let mut projected_evidence = if include.evidence || include.close {
            projected_evidence_summary(store, project_id, state_version, task)?
        } else {
            None
        };
        let close_plan = if include.close {
            let plan = close_task::plan_close_task(
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
            close_state = Some(status_close_state(plan.close_state));
            current_close_basis = plan.current_close_basis.clone();
            risk_acceptance_coverage = Some(plan.risk_acceptance_coverage.clone());
            close_blockers = Some(plan.blockers.clone());
            guard_health = plan.guard_health.clone();
            coverage_summary = plan
                .guard_health
                .as_ref()
                .map(close_task::coverage_summary_from_guard_health);
            next_actions.extend(close_next_actions(&plan.blockers));
            Some(plan)
        } else {
            None
        };
        projected_evidence = projected_evidence
            .map(|summary| evidence_summary_for_display(summary, current_close_basis.as_ref()));
        if include.evidence {
            evidence_summary = projected_evidence.clone();
        }
        card_evidence_summary = projected_evidence.clone();
        if (include.evidence || include.close)
            && projected_evidence
                .as_ref()
                .and_then(|summary| summary.evidence_state)
                .is_none()
        {
            prepared_input_available = store
                .has_prepared_artifact_input(&task_id, &utc_timestamp(now))
                .map_err(CorePipelineError::from)?;
        }
        if include.pending_user_judgments {
            let user_channel = UserChannelContext {
                guard_health: close_plan
                    .as_ref()
                    .and_then(|plan| plan.guard_health.as_ref()),
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
                pending_user_judgment_refs: all_pending_user_judgments,
                blocker_refs: blocker_refs.clone(),
                write_ticket_summary: projected_write_ticket,
                evidence_summary: projected_evidence.clone(),
                close_state: close_plan.as_ref().map(|plan| plan.close_state),
                close_blockers: close_plan
                    .as_ref()
                    .map(|plan| plan.blockers.clone())
                    .unwrap_or_default(),
                guard_health: close_plan
                    .as_ref()
                    .and_then(|plan| plan.guard_health.clone()),
                guarantee_display: guarantee_projection.clone(),
            })?;
            if let Some(task_ref) = &state.task_ref {
                next_actions.extend(next_actions_for_state(
                    task_ref,
                    state.active_change_unit_ref.as_ref(),
                ));
            }
            active_task = Some(status_state_summary_value(state, include)?);
        }
    }
    if include.continuity {
        continuity_summary = Some(projected_continuity_summary(
            store,
            state_version,
            STATUS_CONTINUITY_RECORD_LIMIT,
        )?);
    }
    next_actions = unique_next_actions(next_actions);

    let close_blockers_slice = close_blockers.as_deref().unwrap_or(&[]);
    let summary_card = summary_card_for_core(SummaryCardBuild {
        task,
        recording: "read_only",
        profile: profile_summary_text(guard_health.as_ref(), guarantee_projection.as_ref()),
        write_ticket: write_ticket_summary_text(
            include.write_ticket,
            write_ticket_summary.as_ref(),
        ),
        evidence: evidence_summary_text(
            include.evidence || include.close,
            card_evidence_summary.as_ref(),
            prepared_input_available,
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
        next_action: first_next_action(&next_actions, close_blockers_slice),
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
        close_state,
        current_close_basis: include.close.then(|| current_close_basis.into()),
        risk_acceptance_coverage,
        close_blockers,
        guard_health: include.close.then_some(guard_health).flatten(),
        coverage_summary: include.close.then_some(coverage_summary).flatten(),
        guarantee_display: guarantee_projection.map(RequiredNullable::some),
        continuity_summary,
    };
    let mut result_fields = strip_base(serde_json::to_value(result)?)?;
    if let Some(active_task) = active_task {
        result_fields.insert("active_task".to_owned(), active_task);
    }
    Ok(result_fields)
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

fn unique_next_actions(actions: Vec<NextActionSummary>) -> Vec<NextActionSummary> {
    let mut seen = BTreeSet::new();
    actions
        .into_iter()
        .filter(|action| {
            seen.insert(serde_json::to_string(action).unwrap_or_else(|_| String::new()))
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
