use super::*;

impl CoreService {
    /// Executes `harness.status` as a read-only Core result.
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
                request.requested_access_class(),
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
            &prepared.context.verified_surface,
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
    verified_surface: &VerifiedSurfaceContext,
    task: Option<&TaskRecord>,
    include: &StatusInclude,
    now: DateTime<Utc>,
) -> Result<JsonObject, PlanError> {
    let state_version = project_state.state_version;
    let project_id = &envelope.project_id;
    let mut active_task = None;
    let mut pending_user_judgments = Vec::new();
    let mut blocker_refs = Vec::new();
    let mut write_authority_summary = None;
    let mut evidence_summary = None;
    let mut close_state = None;
    let mut current_close_basis = None;
    let mut risk_acceptance_coverage = None;
    let mut close_blockers = None;
    let mut next_actions = Vec::new();
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
        .map(|profile| guarantee_display_from_profile(profile, verified_surface, state_version));

    if let Some(task) = task {
        let task_id = TaskId::new(task.task_id.clone());
        let current_change_unit = store
            .current_change_unit(&task_id)
            .map_err(CorePipelineError::from)?;
        let all_pending_user_judgments =
            projected_pending_user_judgment_refs(store, &task_id, state_version)?;
        if include.pending_user_judgments {
            pending_user_judgments = all_pending_user_judgments.clone();
        }
        blocker_refs = projected_blocker_refs(store, &task_id, state_version)?;
        let projected_write_authority = if include.write_authority {
            projected_write_authority_summary(
                store,
                &task_id,
                state_version,
                now,
                guarantee_projection.clone(),
            )?
        } else {
            None
        };
        write_authority_summary = projected_write_authority.clone();
        let projected_evidence = if include.evidence {
            projected_evidence_summary(store, project_id, state_version, task)?
        } else {
            None
        };
        if include.evidence {
            evidence_summary = projected_evidence.clone();
        }
        let close_plan = if include.close {
            let plan = close_task::plan_close_task(
                store,
                project_state,
                Some(verified_surface),
                guarantee_profile.as_ref(),
                CloseTaskRequest {
                    envelope: ToolEnvelope {
                        task_id: Some(task_id.clone()).into(),
                        ..envelope.clone()
                    },
                    task_id: task_id.clone(),
                    intent: CloseIntent::Check,
                    close_reason: RequiredNullable::null(),
                    superseding_task_id: RequiredNullable::null(),
                    user_note: RequiredNullable::null(),
                },
                &utc_timestamp(now),
            )?;
            close_state = Some(status_close_state(plan.close_state));
            current_close_basis = plan.current_close_basis.clone();
            risk_acceptance_coverage = Some(plan.risk_acceptance_coverage.clone());
            close_blockers = Some(plan.blockers.clone());
            next_actions.extend(close_next_actions(&plan.blockers));
            Some(plan)
        } else {
            None
        };
        if include.task {
            let state = build_state_summary(SummaryBuild {
                project_id,
                state_version,
                task,
                current_change_unit: current_change_unit.as_ref(),
                pending_user_judgment_refs: all_pending_user_judgments,
                blocker_refs: blocker_refs.clone(),
                write_authority_summary: projected_write_authority,
                evidence_summary: projected_evidence,
                close_state: close_plan.as_ref().map(|plan| plan.close_state),
                close_blockers: close_plan
                    .as_ref()
                    .map(|plan| plan.blockers.clone())
                    .unwrap_or_default(),
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
    next_actions = unique_next_actions(next_actions);

    let result = volicord_types::StatusResult {
        base: placeholder_base(),
        active_task: None,
        status_summary: if task.is_some() {
            "Current Task state is available.".to_owned()
        } else {
            "No current Task is selected.".to_owned()
        },
        next_actions,
        pending_user_judgments,
        blocker_refs,
        write_authority_summary,
        evidence_summary: include.evidence.then(|| evidence_summary.into()),
        close_state,
        current_close_basis: include.close.then(|| current_close_basis.into()),
        risk_acceptance_coverage,
        close_blockers,
        guarantee_display: guarantee_projection.map(RequiredNullable::some),
    };
    let mut result_fields = strip_base(serde_json::to_value(result)?)?;
    if let Some(active_task) = active_task {
        result_fields.insert("active_task".to_owned(), active_task);
    }
    Ok(result_fields)
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
    if !include.write_authority {
        object.remove("write_authority_summary");
    }
    if !include.evidence {
        object.remove("evidence_summary");
    }
    if !include.close {
        object.remove("close_state");
        object.remove("close_blockers");
    }
    if !include.guarantees {
        object.remove("guarantee_display");
    }
    Ok(value)
}
