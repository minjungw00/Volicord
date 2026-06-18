use super::*;

impl CoreService {
    /// Executes `harness.close_task` through close-readiness and terminal transition rules.
    pub fn close_task(
        &self,
        request: CloseTaskRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        if let Some(envelope_task_id) = &request.envelope.task_id {
            if envelope_task_id != &request.task_id {
                return validation_rejected(
                    request.envelope.dry_run,
                    None,
                    "task_id",
                    "envelope.task_id must match CloseTaskRequest.task_id",
                );
            }
        } else {
            return validation_rejected(
                request.envelope.dry_run,
                None,
                "envelope.task_id",
                "close_task requires envelope.task_id to identify the Task being closed",
            );
        }
        if let Some(response) = validate_close_intent_fields(&request)? {
            return Ok(response);
        }
        let close_policy = close_task_policy(&request);
        let prepared = match prepare_or_response(
            self,
            MethodName::CloseTask,
            request.envelope.clone(),
            request_json,
            invocation,
            close_policy,
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        if request.intent != CloseIntent::Check {
            if let Some(response) = reject_stale_close_write_authorization(
                &prepared.store,
                &prepared.context.project_state,
                &request,
            )? {
                return Ok(response);
            }
        }

        if request.intent == CloseIntent::Check {
            let plan = match plan_close_task(
                &prepared.store,
                &prepared.context.project_state,
                request.clone(),
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
            return self.execute_prepared_request(
                prepared,
                OwnerPipelineBranch::ReadOnly {
                    result_fields: plan.result_fields,
                },
            );
        }

        if request.envelope.dry_run {
            return self.execute_prepared_request(
                prepared,
                OwnerPipelineBranch::DryRunPreview {
                    dry_run_summary: close_task_dry_run_summary(request.intent),
                },
            );
        }

        let plan = match plan_close_task(
            &prepared.store,
            &prepared.context.project_state,
            request.clone(),
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

        if !plan.blockers.is_empty() {
            return self.execute_prepared_request(
                prepared,
                OwnerPipelineBranch::NoEffectResult {
                    result_fields: plan.result_fields,
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

fn close_task_policy(request: &CloseTaskRequest) -> MethodPolicy {
    let task = TaskRequirement::Exact(request.task_id.clone());
    if request.intent == CloseIntent::Check {
        MethodPolicy::exact(
            AccessClass::ReadStatus,
            task,
            ReplayPolicy::None,
            FreshnessPolicy::None,
            MethodEffectPolicy::ReadOnly,
        )
    } else {
        mutation_method_policy(AccessClass::CoreMutation, task, request.envelope.dry_run)
    }
}

fn validate_close_intent_fields(
    request: &CloseTaskRequest,
) -> CoreResult<Option<PipelineResponse>> {
    let invalid = |field, message| {
        validation_rejected(request.envelope.dry_run, None, field, message).map(Some)
    };
    match request.intent {
        CloseIntent::Check => {
            if request.close_reason.is_some() {
                return invalid("close_reason", "intent=check must not include close_reason");
            }
            if request.superseding_task_id.is_some() {
                return invalid(
                    "superseding_task_id",
                    "intent=check must not include superseding_task_id",
                );
            }
        }
        CloseIntent::Complete => {
            if !matches!(
                request.close_reason,
                Some(CloseReason::CompletedSelfChecked | CloseReason::CompletedWithRiskAccepted)
            ) {
                return invalid(
                    "close_reason",
                    "intent=complete requires a completion close_reason",
                );
            }
            if request.superseding_task_id.is_some() {
                return invalid(
                    "superseding_task_id",
                    "intent=complete must not include superseding_task_id",
                );
            }
        }
        CloseIntent::Cancel => {
            if request.close_reason != Some(CloseReason::Cancelled) {
                return invalid(
                    "close_reason",
                    "intent=cancel requires close_reason=cancelled",
                );
            }
            if request.superseding_task_id.is_some() {
                return invalid(
                    "superseding_task_id",
                    "intent=cancel must not include superseding_task_id",
                );
            }
        }
        CloseIntent::Supersede => {
            if request.close_reason != Some(CloseReason::Superseded) {
                return invalid(
                    "close_reason",
                    "intent=supersede requires close_reason=superseded",
                );
            }
            let Some(superseding_task_id) = &request.superseding_task_id else {
                return invalid(
                    "superseding_task_id",
                    "intent=supersede requires superseding_task_id",
                );
            };
            if superseding_task_id == &request.task_id {
                return invalid(
                    "superseding_task_id",
                    "superseding_task_id must identify a different Task",
                );
            }
        }
    }
    Ok(None)
}

fn reject_stale_close_write_authorization(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
) -> CoreResult<Option<PipelineResponse>> {
    let active_write_authorizations = store
        .active_write_authorizations(&request.task_id)
        .map_err(CorePipelineError::from)?;
    Ok(active_write_authorizations
        .iter()
        .find(|record| record.basis_state_version != project_state.state_version)
        .map(|record| {
            stale_write_authorization_basis_response(
                &request.envelope,
                record,
                project_state.state_version,
            )
        }))
}

fn close_task_dry_run_summary(intent: CloseIntent) -> DryRunSummary {
    let (action, description) = match intent {
        CloseIntent::Check => (
            "would_check",
            "Close readiness check would read the current Task state.",
        ),
        CloseIntent::Complete => (
            "would_complete",
            "Close task would attempt the complete terminal transition.",
        ),
        CloseIntent::Cancel => (
            "would_cancel",
            "Close task would attempt the cancel terminal transition.",
        ),
        CloseIntent::Supersede => (
            "would_supersede",
            "Close task would attempt the supersede terminal transition.",
        ),
    };
    dry_run_summary("task", action, description, Vec::new())
}

fn plan_close_task(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: CloseTaskRequest,
) -> Result<CloseTaskPlan, PlanError> {
    let context = load_close_task_context(store, project_state, &request)?;
    let mut blockers = terminal_close_blockers(store, project_state, &request, &context)?;
    if matches!(request.intent, CloseIntent::Check | CloseIntent::Complete) {
        blockers.extend(completion_close_blockers(
            store,
            project_state,
            &request,
            &context,
        )?);
    }

    let committed_terminal = request.intent != CloseIntent::Check && blockers.is_empty();
    let response_state_version = if committed_terminal {
        project_state.state_version + 1
    } else {
        project_state.state_version
    };
    let close_state = match request.intent {
        CloseIntent::Check => {
            if blockers.is_empty() {
                CloseState::Ready
            } else {
                CloseState::Blocked
            }
        }
        CloseIntent::Complete => {
            if blockers.is_empty() {
                CloseState::Closed
            } else {
                CloseState::Blocked
            }
        }
        CloseIntent::Cancel => {
            if blockers.is_empty() {
                CloseState::Cancelled
            } else {
                CloseState::Blocked
            }
        }
        CloseIntent::Supersede => {
            if blockers.is_empty() {
                CloseState::Superseded
            } else {
                CloseState::Blocked
            }
        }
    };

    let mut synthetic_task = context.task.clone();
    let mut storage_mutations = Vec::new();
    let mut event_kind = String::new();
    let mut event_payload = Map::new();
    let closed_at = if committed_terminal {
        Some(store.current_timestamp().map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?)
    } else {
        None
    };

    if let Some(closed_at) = &closed_at {
        let terminal = close_terminal_storage(request.intent);
        let close_summary_json = terminal_close_summary_json(&context.task, &request, closed_at)?;
        synthetic_task.lifecycle_phase = terminal.lifecycle_phase.to_owned();
        synthetic_task.result = Some(terminal.result.to_owned());
        synthetic_task.close_summary_json = close_summary_json.clone();
        synthetic_task.closed_at = Some(closed_at.clone());
        storage_mutations.push(CoreStorageMutation::CloseTask(TaskCloseUpdate {
            task_id: request.task_id.as_str().to_owned(),
            lifecycle_phase: terminal.lifecycle_phase.to_owned(),
            result: terminal.result.to_owned(),
            close_summary_json,
            closed_at: closed_at.clone(),
        }));
        if request.intent == CloseIntent::Supersede {
            if let Some(superseding_task_id) = &request.superseding_task_id {
                storage_mutations.push(CoreStorageMutation::SetActiveTask {
                    task_id: superseding_task_id.as_str().to_owned(),
                });
            }
        }
        event_kind = terminal.event_kind.to_owned();
        event_payload = object_from_value(json!({
            "task_id": request.task_id,
            "intent": request.intent,
            "close_reason": request.close_reason,
            "superseding_task_id": request.superseding_task_id,
            "user_note": request.user_note,
            "closed_at": closed_at
        }))?;
    }

    let mut state = build_state_summary(SummaryBuild {
        project_id: &request.envelope.project_id,
        state_version: response_state_version,
        task: &synthetic_task,
        current_change_unit: context.current_change_unit.as_ref(),
        pending_user_judgment_refs: context.pending_user_judgment_refs.clone(),
        blocker_refs: context.blocker_refs.clone(),
        active_write_authorization: None,
        options: SummaryOptions::mutation(),
    })?;
    state.evidence_summary = context.evidence_summary.clone();
    state.close_state = Some(close_state);
    state.close_blockers = blockers.clone();

    let result = CloseTaskResult {
        base: placeholder_base(),
        close_state,
        state,
        blockers: blockers.clone(),
        evidence_summary: context.evidence_summary.clone(),
        artifact_refs: context.artifact_refs.clone(),
    };

    Ok(CloseTaskPlan {
        task_id: request.task_id,
        change_unit_id: context
            .current_change_unit
            .as_ref()
            .map(|record| ChangeUnitId::new(record.change_unit_id.clone())),
        storage_mutations,
        event_kind,
        event_payload,
        result_fields: strip_base(serde_json::to_value(result)?)?,
        blockers,
    })
}

struct CloseTerminalStorage {
    lifecycle_phase: &'static str,
    result: &'static str,
    event_kind: &'static str,
}

fn close_terminal_storage(intent: CloseIntent) -> CloseTerminalStorage {
    match intent {
        CloseIntent::Complete => CloseTerminalStorage {
            lifecycle_phase: "completed",
            result: "completed",
            event_kind: "task_completed",
        },
        CloseIntent::Cancel => CloseTerminalStorage {
            lifecycle_phase: "cancelled",
            result: "cancelled",
            event_kind: "task_cancelled",
        },
        CloseIntent::Supersede => CloseTerminalStorage {
            lifecycle_phase: "superseded",
            result: "superseded",
            event_kind: "task_superseded",
        },
        CloseIntent::Check => CloseTerminalStorage {
            lifecycle_phase: "ready",
            result: "none",
            event_kind: "task_close_checked",
        },
    }
}

fn terminal_close_summary_json(
    task: &TaskRecord,
    request: &CloseTaskRequest,
    closed_at: &str,
) -> CoreResult<String> {
    let mut close_summary = parse_json_object(&task.close_summary_json);
    close_summary.insert(
        "close_reason".to_owned(),
        serde_json::to_value(
            request
                .close_reason
                .expect("validated terminal close_reason is present"),
        )?,
    );
    close_summary.insert("closed_at".to_owned(), Value::String(closed_at.to_owned()));
    close_summary.insert("intent".to_owned(), serde_json::to_value(request.intent)?);
    close_summary.insert(
        "user_note".to_owned(),
        request
            .user_note
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    close_summary.insert(
        "superseding_task_id".to_owned(),
        request
            .superseding_task_id
            .as_ref()
            .map(|id| Value::String(id.as_str().to_owned()))
            .unwrap_or(Value::Null),
    );
    serde_json::to_string(&Value::Object(close_summary)).map_err(CorePipelineError::from)
}

fn load_close_task_context(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
) -> Result<CloseTaskContext, PlanError> {
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
    let pending_user_judgment_refs = store
        .pending_user_judgment_refs(&request.task_id, project_state.state_version)
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
    let blocker_refs = store
        .active_blocker_refs(&request.task_id, project_state.state_version)
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
    let evidence_record = store
        .latest_evidence_summary(&request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
    let evidence_summary = close_evidence_summary(
        evidence_record.as_ref(),
        &task,
        &request.envelope.project_id,
        &request.task_id,
        project_state.state_version,
    )?;
    let artifact_refs = evidence_summary
        .as_ref()
        .map(|summary| summary.artifact_refs.clone())
        .unwrap_or_default();

    Ok(CloseTaskContext {
        task,
        current_change_unit,
        pending_user_judgment_refs,
        blocker_refs,
        evidence_summary,
        artifact_refs,
    })
}

fn terminal_close_blockers(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    context: &CloseTaskContext,
) -> Result<Vec<CloseReadinessBlocker>, PlanError> {
    let mut blockers = Vec::new();
    let task_ref = task_ref_for_close(request, project_state.state_version);
    if is_terminal_lifecycle(&context.task.lifecycle_phase)
        || project_state
            .active_task_id
            .as_deref()
            .is_some_and(|active_task_id| active_task_id != request.task_id.as_str())
    {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::Task,
            "task_not_closeable",
            "The addressed Task is not the current non-terminal Task.",
            vec![task_ref.clone()],
            vec![close_next_action(
                "Review the current Task before closing.",
                vec![task_ref.clone()],
            )],
        ));
    }

    if request.intent == CloseIntent::Supersede {
        let superseding_ref = request.superseding_task_id.as_ref().map(|task_id| {
            state_ref(
                StateRecordKind::Task,
                task_id.as_str(),
                &request.envelope.project_id,
                Some(task_id),
                Some(project_state.state_version),
            )
        });
        let replacement = request
            .superseding_task_id
            .as_ref()
            .map(|task_id| {
                store.task_record(task_id).map_err(|error| {
                    PlanError::Response(Box::new(store_error_response(
                        &request.envelope,
                        project_state,
                        error,
                    )))
                })
            })
            .transpose()?
            .flatten();
        if replacement
            .as_ref()
            .map(|task| is_terminal_lifecycle(&task.lifecycle_phase))
            .unwrap_or(true)
        {
            blockers.push(close_blocker(
                CloseReadinessBlockerCategory::Task,
                "task_not_closeable",
                "superseding_task_id must identify a non-terminal Task in this project.",
                superseding_ref.into_iter().collect(),
                Vec::new(),
            ));
        }
    }

    if recovery_required(context) {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::Recovery,
            "recovery_required",
            "A recovery constraint or active blocker must be resolved before this terminal transition.",
            context.blocker_refs.clone(),
            vec![close_next_action(
                "Resolve recovery blockers before closing the Task.",
                context.blocker_refs.clone(),
            )],
        ));
    }

    Ok(blockers)
}

fn completion_close_blockers(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    context: &CloseTaskContext,
) -> Result<Vec<CloseReadinessBlocker>, PlanError> {
    let mut blockers = Vec::new();
    let task_ref = task_ref_for_close(request, project_state.state_version);
    let change_unit_ref = context.current_change_unit.as_ref().map(|record| {
        change_unit_ref(
            &request.envelope.project_id,
            &request.task_id,
            record,
            project_state.state_version,
        )
    });

    if context
        .current_change_unit
        .as_ref()
        .map(|record| record.status != "active" || !record.is_current)
        .unwrap_or(true)
    {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::Scope,
            "missing_active_change_unit",
            "Completion requires a current active Change Unit.",
            vec![task_ref.clone()],
            vec![NextActionSummary {
                action_kind: NextActionKind::UpdateScope,
                owner_method: Some(MethodName::UpdateScope),
                label: "Create or restore the current active Change Unit.".to_owned(),
                blocking_question: None,
                required_refs: vec![task_ref.clone()],
            }],
        ));
    }

    if !context.pending_user_judgment_refs.is_empty() {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::UserJudgment,
            "pending_user_judgment",
            "A user-owned judgment required before close is still pending.",
            context.pending_user_judgment_refs.clone(),
            vec![NextActionSummary {
                action_kind: NextActionKind::RecordUserJudgment,
                owner_method: Some(MethodName::RecordUserJudgment),
                label: "Resolve pending user-owned judgments required for close.".to_owned(),
                blocking_question: None,
                required_refs: context.pending_user_judgment_refs.clone(),
            }],
        ));
    }

    if sensitive_approval_required(context)
        && !has_resolved_judgment(store, project_state, request, "sensitive_approval")?
    {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::SensitiveApproval,
            "missing_sensitive_approval",
            "A documented sensitive-action approval required for close is missing.",
            change_unit_ref.clone().into_iter().collect(),
            vec![NextActionSummary {
                action_kind: NextActionKind::RequestUserJudgment,
                owner_method: Some(MethodName::RequestUserJudgment),
                label: "Request the user-owned sensitive-action approval.".to_owned(),
                blocking_question: None,
                required_refs: vec![task_ref.clone()],
            }],
        ));
    }

    for record in store
        .active_write_authorizations(&request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .iter()
        .filter(|record| record.basis_state_version != project_state.state_version)
    {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::WriteCompatibility,
            "write_authorization_stale",
            "An active Write Authorization is stale against the current state version.",
            vec![write_authorization_ref(record, project_state.state_version)],
            vec![NextActionSummary {
                action_kind: NextActionKind::PrepareWrite,
                owner_method: Some(MethodName::PrepareWrite),
                label: "Refresh write compatibility before completing the Task.".to_owned(),
                blocking_question: None,
                required_refs: vec![task_ref.clone()],
            }],
        ));
    }

    if baseline_stale_for_close(context) {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::Baseline,
            "baseline_stale",
            "The close basis marks the baseline as stale.",
            change_unit_ref.clone().into_iter().collect(),
            vec![NextActionSummary {
                action_kind: NextActionKind::UpdateScope,
                owner_method: Some(MethodName::UpdateScope),
                label: "Refresh the current scope or close basis before completing the Task."
                    .to_owned(),
                blocking_question: None,
                required_refs: vec![task_ref.clone()],
            }],
        ));
    }

    let unsupported_items = unsupported_close_evidence_items(context.evidence_summary.as_ref());
    if !unsupported_items.is_empty() {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::Evidence,
            "evidence_claim_unsupported",
            "One or more required close evidence claims are unsupported.",
            unsupported_items
                .iter()
                .flat_map(|item| item.gap_refs.clone())
                .collect(),
            vec![NextActionSummary {
                action_kind: NextActionKind::RecordRun,
                owner_method: Some(MethodName::RecordRun),
                label: "Record evidence that supports the required close claims.".to_owned(),
                blocking_question: None,
                required_refs: change_unit_ref.clone().into_iter().collect(),
            }],
        ));
    }

    let unavailable_artifacts = unavailable_close_artifact_refs(
        store,
        project_state,
        request,
        context.evidence_summary.as_ref(),
    )?;
    if !unavailable_artifacts.is_empty() {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::ArtifactAvailability,
            "artifact_unavailable",
            "A required close artifact is missing, unavailable, or incompatible with storage.",
            unavailable_artifacts,
            vec![NextActionSummary {
                action_kind: NextActionKind::RecordRun,
                owner_method: Some(MethodName::RecordRun),
                label: "Record or repair the artifact supporting close evidence.".to_owned(),
                blocking_question: None,
                required_refs: vec![task_ref.clone()],
            }],
        ));
    }

    if !has_resolved_judgment_with_answer(
        store,
        project_state,
        request,
        "final_acceptance",
        |resolution| resolution.answer.final_acceptance.is_some(),
    )? {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::FinalAcceptance,
            "missing_final_acceptance",
            "Final acceptance is required before completing the Task.",
            vec![task_ref.clone()],
            vec![NextActionSummary {
                action_kind: NextActionKind::RequestUserJudgment,
                owner_method: Some(MethodName::RequestUserJudgment),
                label: "Request final acceptance from the user.".to_owned(),
                blocking_question: None,
                required_refs: vec![task_ref.clone()],
            }],
        ));
    }

    let residual_risk = residual_risk_state(context);
    if residual_risk.known && !residual_risk.visible {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::ResidualRiskVisibility,
            "residual_risk_not_visible",
            "Residual risk exists but is not visible in the close basis.",
            vec![task_ref.clone()],
            vec![NextActionSummary {
                action_kind: NextActionKind::RequestUserJudgment,
                owner_method: Some(MethodName::RequestUserJudgment),
                label: "Make residual risk visible before requesting acceptance.".to_owned(),
                blocking_question: None,
                required_refs: vec![task_ref.clone()],
            }],
        ));
    }
    if residual_risk.known
        && residual_risk.visible
        && !has_residual_risk_acceptance(store, project_state, request)?
    {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::ResidualRiskAcceptance,
            "missing_residual_risk_acceptance",
            "Visible residual risk requires distinct residual-risk acceptance.",
            vec![task_ref.clone()],
            vec![NextActionSummary {
                action_kind: NextActionKind::RequestUserJudgment,
                owner_method: Some(MethodName::RequestUserJudgment),
                label: "Request residual-risk acceptance from the user.".to_owned(),
                blocking_question: None,
                required_refs: vec![task_ref],
            }],
        ));
    }

    Ok(blockers)
}

fn close_evidence_summary(
    record: Option<&EvidenceSummaryRecord>,
    task: &TaskRecord,
    project_id: &ProjectId,
    task_id: &TaskId,
    state_version: u64,
) -> CoreResult<Option<EvidenceSummary>> {
    let policy = task_completion_policy(task);
    let mut required_claims = sorted_unique(policy.required_claims);
    if policy.evidence_required && required_claims.is_empty() {
        required_claims.push("completion_evidence".to_owned());
    }
    let required_set = required_claims.iter().cloned().collect::<BTreeSet<_>>();
    let mut coverage_items = record
        .map(|record| {
            parse_json_text::<Vec<EvidenceCoverageItem>>(
                "evidence_summaries.coverage_json",
                &record.coverage_json,
            )
        })
        .transpose()?
        .unwrap_or_default();
    for item in &mut coverage_items {
        if required_set.contains(&item.claim) {
            item.required_for_close = true;
        }
    }
    for claim in &required_set {
        if !coverage_items.iter().any(|item| item.claim == *claim) {
            coverage_items.push(EvidenceCoverageItem {
                claim: claim.clone(),
                required_for_close: true,
                coverage_state: EvidenceCoverageState::Unsupported,
                supporting_refs: Vec::new(),
                supporting_artifact_refs: Vec::new(),
                gap_refs: Vec::new(),
            });
        }
    }
    if coverage_items.is_empty() && !policy.evidence_required {
        return Ok(None);
    }
    let artifact_refs = unique_artifact_refs(
        coverage_items
            .iter()
            .flat_map(|item| item.supporting_artifact_refs.clone())
            .collect(),
    );
    let status = if coverage_items.is_empty() {
        record
            .map(|record| parse_storage_value("evidence_summaries.status", &record.status))
            .transpose()?
            .unwrap_or(EvidenceStatus::Unknown)
    } else {
        evidence_status_for_items(&coverage_items)
    };
    let updated_by_run_ref = record.and_then(|record| {
        string_member(
            &parse_json_object(&record.metadata_json),
            "updated_by_run_id",
        )
        .map(|run_id| {
            state_ref(
                StateRecordKind::Run,
                &run_id,
                project_id,
                Some(task_id),
                Some(state_version),
            )
        })
    });

    Ok(Some(EvidenceSummary {
        status,
        completion_policy: CompletionPolicy {
            evidence_required: policy.evidence_required || !required_claims.is_empty(),
            required_claims,
        },
        coverage_items,
        artifact_refs,
        updated_by_run_ref,
    }))
}

fn task_completion_policy(task: &TaskRecord) -> CompletionPolicy {
    let object = parse_json_object(&task.completion_policy_json);
    let required_claims = string_array_member(&object, "required_claims");
    CompletionPolicy {
        evidence_required: object
            .get("evidence_required")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || !required_claims.is_empty(),
        required_claims,
    }
}

fn unsupported_close_evidence_items(
    evidence_summary: Option<&EvidenceSummary>,
) -> Vec<&EvidenceCoverageItem> {
    evidence_summary
        .map(|summary| {
            summary
                .coverage_items
                .iter()
                .filter(|item| {
                    item.required_for_close
                        && !matches!(
                            item.coverage_state,
                            EvidenceCoverageState::Supported | EvidenceCoverageState::NotApplicable
                        )
                })
                .collect()
        })
        .unwrap_or_default()
}

fn unavailable_close_artifact_refs(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    evidence_summary: Option<&EvidenceSummary>,
) -> Result<Vec<StateRecordRef>, PlanError> {
    let mut seen = BTreeSet::new();
    let mut unavailable = Vec::new();
    let Some(evidence_summary) = evidence_summary else {
        return Ok(unavailable);
    };
    for artifact_ref in evidence_summary
        .coverage_items
        .iter()
        .filter(|item| item.required_for_close)
        .flat_map(|item| item.supporting_artifact_refs.iter())
    {
        if !seen.insert(artifact_ref.artifact_id.as_str().to_owned()) {
            continue;
        }
        let state_ref = state_ref(
            StateRecordKind::Artifact,
            artifact_ref.artifact_id.as_str(),
            &request.envelope.project_id,
            Some(&request.task_id),
            Some(project_state.state_version),
        );
        if artifact_ref.availability != ArtifactAvailability::Available {
            unavailable.push(state_ref);
            continue;
        }
        let stored = store
            .artifact_record(artifact_ref.artifact_id.as_str())
            .map_err(|error| {
                PlanError::Response(Box::new(store_error_response(
                    &request.envelope,
                    project_state,
                    error,
                )))
            })?;
        let Some(stored) = stored else {
            unavailable.push(state_ref);
            continue;
        };
        let owner_link_exists = store
            .artifact_has_task_owner_link(
                artifact_ref.artifact_id.as_str(),
                request.task_id.as_str(),
            )
            .map_err(|error| {
                PlanError::Response(Box::new(store_error_response(
                    &request.envelope,
                    project_state,
                    error,
                )))
            })?;
        if stored.project_id != request.envelope.project_id.as_str()
            || stored.task_id != request.task_id.as_str()
            || stored.status != "available"
            || stored.sha256.as_deref() != Some(artifact_ref.sha256.as_str())
            || stored.size_bytes != Some(artifact_ref.size_bytes)
            || stored.redaction_state != redaction_state_value(artifact_ref.redaction_state)
            || !owner_link_exists
        {
            unavailable.push(state_ref);
        }
    }
    Ok(unavailable)
}

fn has_resolved_judgment(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    judgment_kind: &str,
) -> Result<bool, PlanError> {
    has_resolved_judgment_with_answer(store, project_state, request, judgment_kind, |_| true)
}

fn has_resolved_judgment_with_answer<F>(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    judgment_kind: &str,
    predicate: F,
) -> Result<bool, PlanError>
where
    F: Fn(&UserJudgmentResolution) -> bool,
{
    let records = store
        .resolved_user_judgment_records(&request.task_id, judgment_kind)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
    for record in records {
        let Some(resolution_json) = &record.resolution_json else {
            continue;
        };
        let resolution: UserJudgmentResolution =
            parse_json_text("user_judgments.resolution_json", resolution_json)?;
        if predicate(&resolution) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn has_residual_risk_acceptance(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
) -> Result<bool, PlanError> {
    has_resolved_judgment_with_answer(
        store,
        project_state,
        request,
        "residual_risk_acceptance",
        |resolution| {
            resolution.answer.residual_risk_acceptance.is_some()
                || resolution
                    .accepted_risks
                    .iter()
                    .any(|risk| risk.accepted_for_close)
        },
    )
}

fn sensitive_approval_required(context: &CloseTaskContext) -> bool {
    let close_summary = parse_json_object(&context.task.close_summary_json);
    if !string_array_member(&close_summary, "required_sensitive_categories").is_empty()
        || !string_array_member(&close_summary, "sensitive_categories").is_empty()
    {
        return true;
    }
    context.current_change_unit.as_ref().is_some_and(|record| {
        let close_basis = parse_json_object(&record.close_basis_json);
        !string_array_member(&close_basis, "required_sensitive_categories").is_empty()
            || !string_array_member(&close_basis, "sensitive_categories").is_empty()
    })
}

fn baseline_stale_for_close(context: &CloseTaskContext) -> bool {
    let close_summary = parse_json_object(&context.task.close_summary_json);
    if bool_member(&close_summary, "baseline_stale")
        || string_member(&close_summary, "baseline_status").as_deref() == Some("stale")
    {
        return true;
    }
    context.current_change_unit.as_ref().is_some_and(|record| {
        let close_basis = parse_json_object(&record.close_basis_json);
        bool_member(&close_basis, "baseline_stale")
            || string_member(&close_basis, "baseline_status").as_deref() == Some("stale")
    })
}

fn recovery_required(context: &CloseTaskContext) -> bool {
    if !context.blocker_refs.is_empty() {
        return true;
    }
    let close_summary = parse_json_object(&context.task.close_summary_json);
    if bool_member(&close_summary, "recovery_required") {
        return true;
    }
    context.current_change_unit.as_ref().is_some_and(|record| {
        bool_member(
            &parse_json_object(&record.lifecycle_json),
            "recovery_required",
        ) || bool_member(
            &parse_json_object(&record.close_basis_json),
            "recovery_required",
        )
    })
}

#[derive(Debug, Clone, Copy)]
struct ResidualRiskState {
    known: bool,
    visible: bool,
}

fn residual_risk_state(context: &CloseTaskContext) -> ResidualRiskState {
    let close_summary = parse_json_object(&context.task.close_summary_json);
    let visible = json_array_nonempty_member(&close_summary, "visible_risks")
        || bool_member(&close_summary, "residual_risk_visible");
    let known = visible
        || json_array_nonempty_member(&close_summary, "residual_risks")
        || bool_member(&close_summary, "residual_risk_present");
    ResidualRiskState { known, visible }
}

fn task_ref_for_close(request: &CloseTaskRequest, state_version: u64) -> StateRecordRef {
    state_ref(
        StateRecordKind::Task,
        request.task_id.as_str(),
        &request.envelope.project_id,
        Some(&request.task_id),
        Some(state_version),
    )
}
