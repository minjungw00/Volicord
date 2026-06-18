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
                AccessClass::ReadStatus,
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
            &request.envelope.project_id,
            prepared.context.project_state.state_version,
            task.as_ref(),
            &request.include,
        ) {
            Ok(result_fields) => result_fields,
            Err(error) => {
                return core_error_response(&request.envelope, Some(state_version), error)
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
    project_id: &ProjectId,
    state_version: u64,
    task: Option<&TaskRecord>,
    include: &StatusInclude,
) -> CoreResult<JsonObject> {
    let active_task = if include.task {
        match task {
            Some(task) => {
                let task_id = TaskId::new(task.task_id.clone());
                let current_change_unit = store.current_change_unit(&task_id)?;
                let pending_refs = if include.pending_user_judgments {
                    stored_refs_to_state_refs(
                        store.pending_user_judgment_refs(&task_id, state_version)?,
                    )
                } else {
                    Vec::new()
                };
                let blocker_refs =
                    stored_refs_to_state_refs(store.active_blocker_refs(&task_id, state_version)?);
                let active_write_auths = if include.write_authority {
                    store.active_write_authorizations(&task_id)?
                } else {
                    Vec::new()
                };
                Some(build_state_summary(SummaryBuild {
                    project_id,
                    state_version,
                    task,
                    current_change_unit: current_change_unit.as_ref(),
                    pending_user_judgment_refs: pending_refs,
                    blocker_refs,
                    active_write_authorization: active_write_auths.first(),
                    options: SummaryOptions::status(include),
                })?)
            }
            None => None,
        }
    } else {
        None
    };

    let pending_user_judgments = active_task
        .as_ref()
        .map(|state| state.pending_user_judgment_refs.clone())
        .unwrap_or_default();
    let blocker_refs = active_task
        .as_ref()
        .map(|state| state.blocker_refs.clone())
        .unwrap_or_default();
    let next_actions = active_task
        .as_ref()
        .map(|state| {
            if let Some(task_ref) = &state.task_ref {
                next_actions_for_state(task_ref, state.active_change_unit_ref.as_ref())
            } else {
                Vec::new()
            }
        })
        .unwrap_or_default();
    let result = harness_types::StatusResult {
        base: placeholder_base(),
        active_task,
        status_summary: if task.is_some() {
            "Current Task state is available.".to_owned()
        } else {
            "No current Task is selected.".to_owned()
        },
        next_actions,
        pending_user_judgments,
        blocker_refs,
        close_state: StatusCloseState::None,
        close_blockers: Vec::new(),
        guarantee_display: None,
    };
    strip_base(serde_json::to_value(result)?)
}
