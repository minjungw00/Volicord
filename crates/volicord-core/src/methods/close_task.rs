use super::*;

impl CoreService {
    /// Executes `harness.close_task` through close-readiness and terminal transition rules.
    pub fn close_task(
        &self,
        request: CloseTaskRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        if let Some(envelope_task_id) = request.envelope.task_id.as_ref() {
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
        let plan_now = utc_timestamp(self.now());

        if request.intent == CloseIntent::Check {
            let guarantee_profile = match prepared.store.project_enforcement_profile() {
                Ok(record) => record.profile,
                Err(error) => {
                    return plan_error_response(
                        &request.envelope,
                        &prepared.context.project_state,
                        PlanError::Core(CorePipelineError::from(error)),
                    )
                }
            };
            let plan = match plan_close_task(
                &prepared.store,
                &prepared.context.project_state,
                Some(&prepared.context.verified_surface),
                Some(&guarantee_profile),
                request.clone(),
                &plan_now,
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

        let guarantee_profile = match prepared.store.project_enforcement_profile() {
            Ok(record) => record.profile,
            Err(error) => {
                return plan_error_response(
                    &request.envelope,
                    &prepared.context.project_state,
                    PlanError::Core(CorePipelineError::from(error)),
                )
            }
        };
        let plan = match plan_close_task(
            &prepared.store,
            &prepared.context.project_state,
            Some(&prepared.context.verified_surface),
            Some(&guarantee_profile),
            request.clone(),
            &plan_now,
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
            request.requested_access_class(),
            task,
            ReplayPolicy::None,
            FreshnessPolicy::None,
            MethodEffectPolicy::ReadOnly,
        )
    } else {
        mutation_method_policy(
            request.requested_access_class(),
            task,
            request.envelope.dry_run,
        )
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
                request.close_reason.as_ref(),
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
            if request.close_reason.as_ref() != Some(&CloseReason::Cancelled) {
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
            if request.close_reason.as_ref() != Some(&CloseReason::Superseded) {
                return invalid(
                    "close_reason",
                    "intent=supersede requires close_reason=superseded",
                );
            }
            let Some(superseding_task_id) = request.superseding_task_id.as_ref() else {
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

pub(super) fn plan_close_task(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_surface: Option<&VerifiedSurfaceContext>,
    guarantee_profile: Option<&ProjectEnforcementProfile>,
    request: CloseTaskRequest,
    now: &UtcTimestamp,
) -> Result<CloseTaskPlan, PlanError> {
    let context = load_close_task_context(store, project_state, &request)?;
    plan_close_task_with_context(
        store,
        project_state,
        verified_surface,
        guarantee_profile,
        request,
        now,
        context,
    )
}

pub(super) fn plan_close_task_with_context(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_surface: Option<&VerifiedSurfaceContext>,
    guarantee_profile: Option<&ProjectEnforcementProfile>,
    request: CloseTaskRequest,
    now: &UtcTimestamp,
    context: CloseTaskContext,
) -> Result<CloseTaskPlan, PlanError> {
    let risk_acceptance_coverage =
        risk_acceptance_coverage(store, project_state, &request, &context)?;
    let mut blockers = terminal_close_blockers(store, project_state, &request, &context, now)?;
    if matches!(request.intent, CloseIntent::Check | CloseIntent::Complete) {
        blockers.extend(completion_close_blockers(
            store,
            project_state,
            &request,
            &context,
            &risk_acceptance_coverage,
            now,
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
        Some(now.clone())
    } else {
        None
    };

    if let Some(closed_at) = &closed_at {
        let terminal = close_terminal_storage(request.intent);
        let close_summary_json = terminal_close_summary_json(&context.task, &request, closed_at)?;
        synthetic_task.lifecycle_phase = terminal.lifecycle_phase.to_owned();
        synthetic_task.result = Some(terminal.result.to_owned());
        synthetic_task.close_summary_json = close_summary_json.clone();
        synthetic_task.closed_at = Some(closed_at.to_string());
        storage_mutations.push(CoreStorageMutation::CloseTask(TaskCloseUpdate {
            task_id: request.task_id.as_str().to_owned(),
            lifecycle_phase: terminal.lifecycle_phase.to_owned(),
            result: terminal.result.to_owned(),
            close_summary_json,
            closed_at: closed_at.to_string(),
        }));
        if request.intent == CloseIntent::Supersede {
            if let Some(superseding_task_id) = request.superseding_task_id.as_ref() {
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

    let guarantee_display = match (verified_surface, guarantee_profile) {
        (Some(surface), Some(profile)) => Some(guarantee_display_from_profile(
            profile,
            surface,
            response_state_version,
        )),
        _ => None,
    };

    let state = build_state_summary(SummaryBuild {
        project_id: &request.envelope.project_id,
        state_version: response_state_version,
        task: &synthetic_task,
        current_change_unit: context.current_change_unit.as_ref(),
        pending_user_judgment_refs: context.pending_user_judgment_refs.clone(),
        blocker_refs: context.blocker_refs.clone(),
        write_authority_summary: projected_write_authority_summary(
            store,
            &request.task_id,
            response_state_version,
            *now.as_datetime(),
            guarantee_display.clone(),
        )?,
        evidence_summary: context.evidence_summary.clone(),
        close_state: Some(close_state),
        close_blockers: blockers.clone(),
        guarantee_display,
    })?;

    let result_state = state.clone();
    let result_current_close_basis = context.current_close_basis.clone();
    let result_risk_acceptance_coverage = risk_acceptance_coverage.clone();
    let result_evidence_summary = context.evidence_summary.clone();
    let result_artifact_refs = context.artifact_refs.clone();
    let result = CloseTaskResult {
        base: placeholder_base(),
        close_state,
        current_close_basis: result_current_close_basis.clone(),
        risk_acceptance_coverage: result_risk_acceptance_coverage.clone(),
        state: result_state.clone(),
        blockers: blockers.clone(),
        evidence_summary: result_evidence_summary.clone(),
        artifact_refs: result_artifact_refs.clone(),
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
        close_state,
        current_close_basis: result_current_close_basis,
        risk_acceptance_coverage: result_risk_acceptance_coverage,
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
    closed_at: &UtcTimestamp,
) -> CoreResult<String> {
    let mut close_summary = decode_required_json_object(
        "tasks",
        task.task_id.clone(),
        "close_summary_json",
        Some(&task.close_summary_json),
    )?;
    close_summary.insert(
        "close_reason".to_owned(),
        serde_json::to_value(
            request
                .close_reason
                .as_ref()
                .expect("validated terminal close_reason is present"),
        )?,
    );
    close_summary.insert("closed_at".to_owned(), serde_json::to_value(closed_at)?);
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
    let task_revision = store
        .task_revision_record(&request.task_id)
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
    let current_close_basis = task_revision.current_close_basis;
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
    let evidence_record = current_close_basis
        .as_ref()
        .and_then(|basis| basis.evidence_summary_ref.as_ref())
        .map(|evidence_ref| {
            store
                .evidence_summary_record(evidence_ref.record_id.as_str())
                .map_err(|error| {
                    PlanError::Response(Box::new(store_error_response(
                        &request.envelope,
                        project_state,
                        error,
                    )))
                })
        })
        .transpose()?
        .flatten();
    let evidence_summary = close_evidence_summary(
        store,
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
        current_close_basis,
        pending_user_judgment_refs,
        blocker_refs,
        evidence_summary,
        artifact_refs,
        pending_judgment_authorities: None,
        resolved_judgment_authorities: None,
    })
}

fn terminal_close_blockers(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    context: &CloseTaskContext,
    now: &UtcTimestamp,
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

    if recovery_required(context)? {
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

    match request.intent {
        CloseIntent::Cancel => {
            if let Some(blocker) =
                cancellation_authority_blocker(store, project_state, request, context)?
            {
                blockers.push(blocker);
            }
        }
        CloseIntent::Supersede => {
            let pending_refs = pending_judgment_refs_for_close_operation(
                store,
                project_state,
                request,
                context,
                JudgmentOperation::CloseSupersede,
                now,
            )?;
            if !pending_refs.is_empty() {
                blockers.push(close_blocker(
                    CloseReadinessBlockerCategory::UserJudgment,
                    "pending_user_judgment",
                    "A user-owned judgment required before superseding this Task is still pending.",
                    pending_refs.clone(),
                    vec![NextActionSummary {
                        action_kind: NextActionKind::RecordUserJudgment,
                        owner_method: Some(MethodName::RecordUserJudgment),
                        label: "Resolve pending user-owned judgments required for supersession."
                            .to_owned(),
                        blocking_question: None,
                        required_refs: pending_refs,
                    }],
                ));
            }
        }
        CloseIntent::Check | CloseIntent::Complete => {}
    }

    Ok(blockers)
}

fn pending_judgment_refs_for_close_operation(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    context: &CloseTaskContext,
    operation: JudgmentOperation,
    now: &UtcTimestamp,
) -> Result<Vec<StateRecordRef>, PlanError> {
    let authorities =
        pending_judgment_authorities_for_context(store, project_state, request, context)?;
    let current_change_unit_id = context
        .current_change_unit
        .as_ref()
        .map(|record| ChangeUnitId::new(record.change_unit_id.clone()));
    let operation_refs = close_operation_refs(request, project_state, context);
    let mut refs = Vec::new();
    for authority in &authorities {
        let blocks = if operation == JudgmentOperation::CloseComplete
            && authority.judgment_kind == JudgmentKind::SensitiveApproval
        {
            pending_sensitive_judgment_blocks_close(
                store,
                request,
                context,
                authority,
                current_change_unit_id.as_ref(),
                &operation_refs,
                now,
            )
        } else {
            let operation_context = JudgmentOperationContext {
                operation,
                task_id: &request.task_id,
                change_unit_id: current_change_unit_id.as_ref(),
                scope_revision: context.task.scope_revision,
                close_basis: context.current_close_basis.as_ref(),
                operation_refs: &operation_refs,
                sensitive_approval: None,
            };
            judgment_blocks_operation(authority, &operation_context)
        };
        if blocks {
            refs.push(state_ref(
                StateRecordKind::UserJudgment,
                &authority.judgment_id,
                &request.envelope.project_id,
                Some(&request.task_id),
                Some(project_state.state_version),
            ));
        }
    }
    Ok(refs)
}

fn pending_judgment_authorities_for_context(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    context: &CloseTaskContext,
) -> Result<Vec<JudgmentAuthority>, PlanError> {
    if let Some(authorities) = &context.pending_judgment_authorities {
        return Ok(authorities.clone());
    }
    pending_judgment_authorities_for_plan(store, project_state, &request.envelope, &request.task_id)
}

fn resolved_judgment_authorities_for_context(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    context: &CloseTaskContext,
    judgment_kind: JudgmentKind,
) -> Result<Vec<JudgmentAuthority>, PlanError> {
    if let Some(authorities) = &context.resolved_judgment_authorities {
        return Ok(authorities
            .iter()
            .filter(|authority| authority.judgment_kind == judgment_kind)
            .cloned()
            .collect());
    }
    resolved_judgment_authorities_for_plan(
        store,
        project_state,
        &request.envelope,
        &request.task_id,
        judgment_kind,
    )
}

fn pending_sensitive_judgment_blocks_close(
    store: &CoreProjectStore,
    request: &CloseTaskRequest,
    context: &CloseTaskContext,
    authority: &JudgmentAuthority,
    current_change_unit_id: Option<&ChangeUnitId>,
    operation_refs: &[StateRecordRef],
    now: &UtcTimestamp,
) -> bool {
    let Some(close_basis) = context.current_close_basis.as_ref() else {
        return false;
    };
    close_basis
        .sensitive_action_requirements
        .iter()
        .any(|close_requirement| {
            let requirement = SensitiveApprovalRequirement {
                task_id: &request.task_id,
                change_unit_id: &close_requirement.change_unit_id,
                scope_revision: context.task.scope_revision,
                operation: &close_requirement.action_kind,
                normalized_paths: &close_requirement.normalized_paths,
                sensitive_categories: &close_requirement.sensitive_categories,
                baseline_ref: close_requirement.baseline_ref.as_ref(),
                required_for: JudgmentRequiredFor::CloseComplete,
                now,
                repo_root: &store.project_record().repo_root,
            };
            let operation_context = JudgmentOperationContext {
                operation: JudgmentOperation::CloseComplete,
                task_id: &request.task_id,
                change_unit_id: current_change_unit_id,
                scope_revision: context.task.scope_revision,
                close_basis: Some(close_basis),
                operation_refs,
                sensitive_approval: Some(&requirement),
            };
            judgment_blocks_operation(authority, &operation_context)
        })
}

fn close_operation_refs(
    request: &CloseTaskRequest,
    project_state: &ProjectStateHeader,
    context: &CloseTaskContext,
) -> Vec<StateRecordRef> {
    let mut refs = vec![task_ref_for_close(request, project_state.state_version)];
    if let Some(change_unit) = context.current_change_unit.as_ref() {
        refs.push(change_unit_ref(
            &request.envelope.project_id,
            &request.task_id,
            change_unit,
            project_state.state_version,
        ));
    }
    if let Some(close_basis) = context.current_close_basis.as_ref() {
        refs.extend(close_basis.result_refs.clone());
        if let Some(evidence_ref) = close_basis.evidence_summary_ref.as_ref() {
            refs.push(evidence_ref.clone());
        }
        for risk in &close_basis.residual_risks {
            refs.extend(risk.source_refs.clone());
        }
    }
    refs
}

fn cancellation_authority_blocker(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    context: &CloseTaskContext,
) -> Result<Option<CloseReadinessBlocker>, PlanError> {
    let current_change_unit_id = context
        .current_change_unit
        .as_ref()
        .map(|record| ChangeUnitId::new(record.change_unit_id.clone()));
    let requirement = CancellationAuthorityRequirement {
        task_id: &request.task_id,
        change_unit_id: current_change_unit_id.as_ref(),
        scope_revision: context.task.scope_revision,
    };
    let authorities = resolved_judgment_authorities_for_context(
        store,
        project_state,
        request,
        context,
        JudgmentKind::Cancellation,
    )?;
    if authorities.iter().any(|authority| {
        judgment_required_for(authority, JudgmentRequiredFor::CloseCancel)
            && current_cancellation_authority(authority, &requirement)
    }) {
        return Ok(None);
    }

    let mut stale_refs = Vec::new();
    let mut rejected_refs = Vec::new();
    for authority in &authorities {
        if !judgment_required_for(authority, JudgmentRequiredFor::CloseCancel) {
            continue;
        }
        let judgment_ref = state_ref(
            StateRecordKind::UserJudgment,
            &authority.judgment_id,
            &request.envelope.project_id,
            Some(&request.task_id),
            Some(project_state.state_version),
        );
        let current_basis_matches = authority.basis.as_ref().is_some_and(|basis| {
            basis.task_id == request.task_id
                && basis.scope_revision == context.task.scope_revision
                && basis.change_unit_id.as_ref() == current_change_unit_id.as_ref()
        });
        if !judgment_has_current_basis(authority) || !current_basis_matches {
            stale_refs.push(judgment_ref);
        } else if authority.resolution_outcome == Some(JudgmentResolutionOutcome::Rejected)
            && authority
                .resolution
                .as_ref()
                .is_some_and(|resolution| resolution.resolved_by_actor_kind == ActorKind::User)
            && verified_user_interaction_provenance(authority)
        {
            rejected_refs.push(judgment_ref);
        }
    }
    if stale_refs.is_empty() {
        stale_refs.extend(non_current_judgment_refs_for_plan(
            store,
            project_state,
            request,
            JudgmentKind::Cancellation,
        )?);
    }

    let task_ref = task_ref_for_close(request, project_state.state_version);
    let (code, message, related_refs) = if !rejected_refs.is_empty() {
        (
            "cancellation_rejected",
            "The current user cancellation judgment rejected cancellation.",
            refs_with_context(vec![task_ref.clone()], rejected_refs),
        )
    } else if !stale_refs.is_empty() {
        (
            "cancellation_judgment_stale",
            "The available cancellation judgment is stale or incompatible with the current Task scope.",
            refs_with_context(vec![task_ref.clone()], stale_refs),
        )
    } else {
        (
            "missing_cancellation_authority",
            "Cancelling the Task requires a current accepted user cancellation judgment.",
            vec![task_ref.clone()],
        )
    };
    Ok(Some(close_blocker(
        CloseReadinessBlockerCategory::UserJudgment,
        code,
        message,
        related_refs,
        vec![NextActionSummary {
            action_kind: NextActionKind::RequestUserJudgment,
            owner_method: Some(MethodName::RequestUserJudgment),
            label: "Request current user cancellation authority.".to_owned(),
            blocking_question: None,
            required_refs: vec![task_ref],
        }],
    )))
}

fn completion_close_blockers(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    context: &CloseTaskContext,
    risk_acceptance_coverage: &[RiskAcceptanceCoverage],
    now: &UtcTimestamp,
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

    if let Some(blocker) = current_close_basis_blocker(store, request, project_state, context)? {
        blockers.push(blocker);
    }

    let close_complete_pending_refs = pending_judgment_refs_for_close_operation(
        store,
        project_state,
        request,
        context,
        JudgmentOperation::CloseComplete,
        now,
    )?;
    if !close_complete_pending_refs.is_empty() {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::UserJudgment,
            "pending_user_judgment",
            "A user-owned judgment required before close is still pending.",
            close_complete_pending_refs.clone(),
            vec![NextActionSummary {
                action_kind: NextActionKind::RecordUserJudgment,
                owner_method: Some(MethodName::RecordUserJudgment),
                label: "Resolve pending user-owned judgments required for close.".to_owned(),
                blocking_question: None,
                required_refs: close_complete_pending_refs,
            }],
        ));
    }

    if sensitive_approval_required(context)?
        && !has_current_sensitive_approval_for_close(store, project_state, request, context, now)?
    {
        let related_refs = refs_with_context(
            change_unit_ref.clone().into_iter().collect(),
            non_current_judgment_refs_for_plan(
                store,
                project_state,
                request,
                JudgmentKind::SensitiveApproval,
            )?,
        );
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::SensitiveApproval,
            "missing_sensitive_approval",
            "A documented sensitive-action approval required for close is missing.",
            related_refs,
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

    if baseline_stale_for_close(context)? {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::Baseline,
            "baseline_stale",
            "The current close basis is stale against the current baseline.",
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

    if let Some(basis) = context.current_close_basis.as_ref() {
        if !basis.recovery_constraints.is_empty() {
            blockers.push(close_blocker(
                CloseReadinessBlockerCategory::Recovery,
                "recovery_required",
                "The current close basis records recovery constraints that must be resolved.",
                vec![task_ref.clone()],
                vec![close_next_action(
                    "Resolve recovery constraints before completing the Task.",
                    vec![task_ref.clone()],
                )],
            ));
        }
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

    let unavailable_artifacts =
        unavailable_close_artifact_refs(store, project_state, request, context)?;
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

    if !has_current_final_acceptance(store, project_state, request, context)? {
        let related_refs = refs_with_context(
            vec![task_ref.clone()],
            non_current_judgment_refs_for_plan(
                store,
                project_state,
                request,
                JudgmentKind::FinalAcceptance,
            )?,
        );
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::FinalAcceptance,
            "missing_final_acceptance",
            "Final acceptance is required before completing the Task.",
            related_refs,
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
        && risk_acceptance_coverage
            .iter()
            .any(|coverage| !coverage.accepted)
    {
        let related_refs = refs_with_context(
            vec![task_ref.clone()],
            non_current_judgment_refs_for_plan(
                store,
                project_state,
                request,
                JudgmentKind::ResidualRiskAcceptance,
            )?,
        );
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::ResidualRiskAcceptance,
            "missing_residual_risk_acceptance",
            "Visible residual risk requires distinct residual-risk acceptance.",
            related_refs,
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

pub(super) fn close_evidence_summary(
    store: &CoreProjectStore,
    record: Option<&EvidenceSummaryRecord>,
    task: &TaskRecord,
    project_id: &ProjectId,
    task_id: &TaskId,
    state_version: u64,
) -> CoreResult<Option<EvidenceSummary>> {
    let policy = task_completion_policy(task)?;
    let mut required_claims = sorted_unique(policy.required_claims);
    if policy.evidence_required && required_claims.is_empty() {
        required_claims.push("completion_evidence".to_owned());
    }
    let required_set = required_claims.iter().cloned().collect::<BTreeSet<_>>();
    let mut coverage_items = record
        .map(|record| {
            decode_required_json::<Vec<EvidenceCoverageItem>>(
                "evidence_summaries",
                record.evidence_summary_id.clone(),
                "coverage_json",
                Some(&record.coverage_json),
            )
        })
        .transpose()?
        .unwrap_or_default();
    if let Some(record) = record {
        let _supporting_refs: Vec<StateRecordRef> = decode_required_json(
            "evidence_summaries",
            record.evidence_summary_id.clone(),
            "supporting_refs_json",
            Some(&record.supporting_refs_json),
        )?;
        let _gap_refs: Vec<StateRecordRef> = decode_required_json(
            "evidence_summaries",
            record.evidence_summary_id.clone(),
            "gap_refs_json",
            Some(&record.gap_refs_json),
        )?;
    }
    for item in &mut coverage_items {
        if required_set.contains(&item.claim) {
            item.required_for_close = true;
        }
        item.supporting_artifact_refs = item
            .supporting_artifact_refs
            .iter()
            .map(|artifact_ref| {
                sanitize_evidence_artifact_ref(
                    store,
                    artifact_ref,
                    project_id,
                    task_id,
                    state_version,
                )
            })
            .collect::<CoreResult<Vec<_>>>()?;
        if item.required_for_close
            && item.coverage_state == EvidenceCoverageState::Supported
            && item.supporting_artifact_refs.iter().any(|artifact_ref| {
                artifact_ref.availability != ArtifactAvailability::Available
                    || artifact_ref.integrity_status != ArtifactIntegrityStatus::Verified
            })
        {
            item.coverage_state = EvidenceCoverageState::Blocked;
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
            .map(|record| {
                parse_owner_storage_value(
                    "evidence_summaries",
                    record.evidence_summary_id.clone(),
                    "status",
                    &record.status,
                )
            })
            .transpose()?
            .unwrap_or(EvidenceStatus::Unknown)
    } else {
        evidence_status_for_items(&coverage_items)
    };
    let updated_by_run_ref = record
        .map(|record| {
            let metadata: PersistedEvidenceMetadata = decode_required_json(
                "evidence_summaries",
                record.evidence_summary_id.clone(),
                "metadata_json",
                Some(&record.metadata_json),
            )?;
            Ok::<_, CorePipelineError>(state_ref(
                StateRecordKind::Run,
                metadata.updated_by_run_id.as_str(),
                project_id,
                Some(task_id),
                Some(state_version),
            ))
        })
        .transpose()?;

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

fn sanitize_evidence_artifact_ref(
    store: &CoreProjectStore,
    artifact_ref: &ArtifactRef,
    project_id: &ProjectId,
    task_id: &TaskId,
    state_version: u64,
) -> CoreResult<ArtifactRef> {
    if artifact_ref.project_id != *project_id || artifact_ref.task_id != *task_id {
        return Ok(unavailable_artifact_ref_from_raw(
            artifact_ref,
            ArtifactAvailability::Unusable,
        ));
    }
    let Some(record) = store.artifact_record(artifact_ref.artifact_id.as_str())? else {
        return Ok(unavailable_artifact_ref_from_raw(
            artifact_ref,
            ArtifactAvailability::Missing,
        ));
    };
    artifact_ref_from_verified_record(
        store,
        &record,
        Some(artifact_ref.display_name.clone()),
        Some(state_version),
    )
}

fn unavailable_artifact_ref_from_raw(
    artifact_ref: &ArtifactRef,
    availability: ArtifactAvailability,
) -> ArtifactRef {
    ArtifactRef {
        artifact_id: artifact_ref.artifact_id.clone(),
        project_id: artifact_ref.project_id.clone(),
        task_id: artifact_ref.task_id.clone(),
        display_name: artifact_ref.display_name.clone(),
        content_type: artifact_ref.content_type.clone(),
        sha256: artifact_ref.sha256.clone(),
        size_bytes: artifact_ref.size_bytes.clone(),
        integrity_status: artifact_ref.integrity_status,
        redaction_state: artifact_ref.redaction_state,
        availability,
        created_by_run_ref: artifact_ref.created_by_run_ref.clone(),
        created_by_surface_id: artifact_ref.created_by_surface_id.clone(),
        created_by_surface_instance_id: artifact_ref.created_by_surface_instance_id.clone(),
        storage_ref: artifact_ref.storage_ref.clone(),
    }
}

fn current_close_basis_blocker(
    store: &CoreProjectStore,
    request: &CloseTaskRequest,
    project_state: &ProjectStateHeader,
    context: &CloseTaskContext,
) -> Result<Option<CloseReadinessBlocker>, PlanError> {
    let task_ref = task_ref_for_close(request, project_state.state_version);
    let Some(basis) = context.current_close_basis.as_ref() else {
        return Ok(Some(close_blocker(
            CloseReadinessBlockerCategory::Task,
            "missing_current_close_basis",
            "Completion requires a current close basis recorded by harness.record_run.",
            vec![task_ref.clone()],
            vec![NextActionSummary {
                action_kind: NextActionKind::RecordRun,
                owner_method: Some(MethodName::RecordRun),
                label: "Record the current result and close basis.".to_owned(),
                blocking_question: None,
                required_refs: vec![task_ref],
            }],
        )));
    };
    let current_change_unit_id = context
        .current_change_unit
        .as_ref()
        .map(|record| record.change_unit_id.as_str());
    let current_baseline = StoredScope::from_task(&context.task)?.baseline_ref;
    if !close_basis_is_current(
        basis,
        &request.task_id,
        current_change_unit_id,
        context.task.scope_revision,
        context.task.close_basis_revision,
        current_baseline.as_deref(),
    ) {
        Ok(Some(close_blocker(
            CloseReadinessBlockerCategory::Scope,
            "stale_current_close_basis",
            "The current close basis is stale against current Task scope.",
            vec![task_ref.clone()],
            vec![NextActionSummary {
                action_kind: NextActionKind::RecordRun,
                owner_method: Some(MethodName::RecordRun),
                label: "Record a fresh close basis for the current scope.".to_owned(),
                blocking_question: None,
                required_refs: vec![task_ref],
            }],
        )))
    } else if let Some(blocker) = incompatible_close_basis_run_refs_blocker(
        store,
        request,
        project_state,
        context,
        basis,
        current_baseline.as_deref(),
    )? {
        Ok(Some(blocker))
    } else {
        Ok(None)
    }
}

fn incompatible_close_basis_run_refs_blocker(
    store: &CoreProjectStore,
    request: &CloseTaskRequest,
    project_state: &ProjectStateHeader,
    context: &CloseTaskContext,
    basis: &CurrentCloseBasis,
    current_baseline: Option<&str>,
) -> Result<Option<CloseReadinessBlocker>, PlanError> {
    let Some(current_change_unit) = context.current_change_unit.as_ref() else {
        return Ok(None);
    };
    let current_change_unit_id = current_change_unit.change_unit_id.as_str();
    let mut seen = BTreeSet::new();
    let mut incompatible_refs = Vec::new();
    for record_ref in close_basis_run_refs(basis) {
        let record_id = record_ref.record_id.as_str();
        if !seen.insert(record_id.to_owned()) {
            continue;
        }
        if record_ref.project_id != request.envelope.project_id
            || record_ref.task_id.as_ref() != Some(&request.task_id)
        {
            incompatible_refs.push(record_ref.clone());
            continue;
        }
        let record = store.run_record(record_id).map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
        if record.as_ref().is_none_or(|record| {
            stored_run_is_not_current_close_basis_compatible(
                record,
                request,
                current_change_unit_id,
                context.task.scope_revision,
                current_baseline,
            )
        }) {
            incompatible_refs.push(record_ref.clone());
        }
    }

    if incompatible_refs.is_empty() {
        Ok(None)
    } else {
        let task_ref = task_ref_for_close(request, project_state.state_version);
        Ok(Some(close_blocker(
            CloseReadinessBlockerCategory::Scope,
            "stale_current_close_basis",
            "The current close basis contains Run refs that are not current for the Task scope.",
            incompatible_refs,
            vec![NextActionSummary {
                action_kind: NextActionKind::RecordRun,
                owner_method: Some(MethodName::RecordRun),
                label: "Record a fresh close basis for the current Run context.".to_owned(),
                blocking_question: None,
                required_refs: vec![task_ref],
            }],
        )))
    }
}

fn close_basis_run_refs(basis: &CurrentCloseBasis) -> Vec<&StateRecordRef> {
    let mut refs = Vec::new();
    refs.push(&basis.source_run_ref);
    refs.extend(
        basis
            .result_refs
            .iter()
            .filter(|record_ref| record_ref.record_kind == StateRecordKind::Run),
    );
    refs.extend(
        basis
            .residual_risks
            .iter()
            .flat_map(|risk| risk.source_refs.iter())
            .filter(|record_ref| record_ref.record_kind == StateRecordKind::Run),
    );
    refs
}

fn stored_run_is_not_current_close_basis_compatible(
    record: &RunRecord,
    request: &CloseTaskRequest,
    current_change_unit_id: &str,
    current_scope_revision: u64,
    current_baseline: Option<&str>,
) -> bool {
    record.project_id != request.envelope.project_id.as_str()
        || record.task_id != request.task_id.as_str()
        || record.change_unit_id.as_deref() != Some(current_change_unit_id)
        || record.scope_revision != current_scope_revision
        || record.baseline_ref.as_deref() != current_baseline
        || record.status != "recorded"
}

fn task_completion_policy(task: &TaskRecord) -> CoreResult<CompletionPolicy> {
    let persisted: PersistedCompletionPolicy = decode_required_json(
        "tasks",
        task.task_id.clone(),
        "completion_policy_json",
        Some(&task.completion_policy_json),
    )?;
    Ok(CompletionPolicy {
        evidence_required: persisted.evidence_required || !persisted.required_claims.is_empty(),
        required_claims: persisted.required_claims,
    })
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
    context: &CloseTaskContext,
) -> Result<Vec<StateRecordRef>, PlanError> {
    let mut seen = BTreeSet::new();
    let mut unavailable = Vec::new();
    if let Some(evidence_summary) = context.evidence_summary.as_ref() {
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
            let stored_available = persistent_artifact_is_verified_current(store, &stored)?;
            let stored_redaction_state: RedactionState = parse_owner_storage_value(
                "artifacts",
                stored.artifact_id.clone(),
                "redaction_state",
                &stored.redaction_state,
            )?;
            let artifact_sha256 = artifact_ref.sha256.as_ref();
            let artifact_size_bytes = artifact_ref.size_bytes.as_ref().copied();
            if stored.project_id != request.envelope.project_id.as_str()
                || stored.task_id != request.task_id.as_str()
                || !stored_available
                || artifact_ref.integrity_status != ArtifactIntegrityStatus::Verified
                || stored.sha256.as_deref() != artifact_sha256.map(String::as_str)
                || stored.size_bytes != artifact_size_bytes
                || stored_redaction_state != artifact_ref.redaction_state
                || !owner_link_exists
            {
                unavailable.push(state_ref);
            }
        }
    }
    if let Some(basis) = context.current_close_basis.as_ref() {
        for record_ref in basis
            .result_refs
            .iter()
            .chain(
                basis
                    .residual_risks
                    .iter()
                    .flat_map(|risk| risk.source_refs.iter()),
            )
            .filter(|record_ref| record_ref.record_kind == StateRecordKind::Artifact)
        {
            if !seen.insert(record_ref.record_id.as_str().to_owned()) {
                continue;
            }
            if close_basis_artifact_ref_unavailable(store, request, record_ref, project_state)? {
                unavailable.push(record_ref.clone());
            }
        }
    }
    Ok(unavailable)
}

fn close_basis_artifact_ref_unavailable(
    store: &CoreProjectStore,
    request: &CloseTaskRequest,
    record_ref: &StateRecordRef,
    project_state: &ProjectStateHeader,
) -> Result<bool, PlanError> {
    let stored = store
        .artifact_record(record_ref.record_id.as_str())
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
    let owner_link_exists = store
        .artifact_has_task_owner_link(record_ref.record_id.as_str(), request.task_id.as_str())
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
    Ok(stored
        .as_ref()
        .map(|record| {
            let available = persistent_artifact_is_verified_current(store, record)?;
            let unavailable = record.project_id != request.envelope.project_id.as_str()
                || record.task_id != request.task_id.as_str()
                || !available
                || !owner_link_exists;
            Ok::<_, CorePipelineError>(unavailable)
        })
        .transpose()?
        .unwrap_or(true))
}

fn has_current_final_acceptance(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    context: &CloseTaskContext,
) -> Result<bool, PlanError> {
    let Some(close_basis) = context.current_close_basis.as_ref() else {
        return Ok(false);
    };
    let requirement = final_acceptance_requirement(close_basis);
    let authorities = resolved_judgment_authorities_for_context(
        store,
        project_state,
        request,
        context,
        JudgmentKind::FinalAcceptance,
    )?;
    Ok(authorities
        .iter()
        .any(|authority| current_final_acceptance(authority, &requirement)))
}

fn has_current_sensitive_approval_for_close(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    context: &CloseTaskContext,
    now: &UtcTimestamp,
) -> Result<bool, PlanError> {
    let Some(close_basis) = context.current_close_basis.as_ref() else {
        return Ok(false);
    };
    if close_basis.sensitive_action_requirements.is_empty() {
        return Ok(true);
    }
    let authorities = resolved_judgment_authorities_for_context(
        store,
        project_state,
        request,
        context,
        JudgmentKind::SensitiveApproval,
    )?;
    Ok(close_basis
        .sensitive_action_requirements
        .iter()
        .all(|close_requirement| {
            if close_requirement.change_unit_id != close_basis.change_unit_id {
                return false;
            }
            let requirement = SensitiveApprovalRequirement {
                task_id: &request.task_id,
                change_unit_id: &close_requirement.change_unit_id,
                scope_revision: context.task.scope_revision,
                operation: &close_requirement.action_kind,
                normalized_paths: &close_requirement.normalized_paths,
                sensitive_categories: &close_requirement.sensitive_categories,
                baseline_ref: close_requirement.baseline_ref.as_ref(),
                required_for: JudgmentRequiredFor::CloseComplete,
                now,
                repo_root: &store.project_record().repo_root,
            };
            authorities
                .iter()
                .any(|authority| current_sensitive_approval(authority, &requirement))
        }))
}

fn risk_acceptance_coverage(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    context: &CloseTaskContext,
) -> Result<Vec<RiskAcceptanceCoverage>, PlanError> {
    let Some(basis) = context.current_close_basis.as_ref() else {
        return Ok(Vec::new());
    };
    let authorities = resolved_judgment_authorities_for_context(
        store,
        project_state,
        request,
        context,
        JudgmentKind::ResidualRiskAcceptance,
    )?;
    Ok(current_residual_risk_acceptance_coverage(
        &request.envelope.project_id,
        &request.task_id,
        project_state.state_version,
        basis,
        &authorities,
    ))
}

fn non_current_judgment_refs_for_plan(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    judgment_kind: JudgmentKind,
) -> Result<Vec<StateRecordRef>, PlanError> {
    let kind = storage_value(judgment_kind)?;
    store
        .non_current_user_judgment_refs(&request.task_id, &kind, project_state.state_version)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })
        .map(stored_refs_to_state_refs)
}

fn refs_with_context(
    mut refs: Vec<StateRecordRef>,
    context_refs: Vec<StateRecordRef>,
) -> Vec<StateRecordRef> {
    refs.extend(context_refs);
    refs
}

fn sensitive_approval_required(context: &CloseTaskContext) -> CoreResult<bool> {
    Ok(context
        .current_close_basis
        .as_ref()
        .map(|basis| !basis.sensitive_action_requirements.is_empty())
        .unwrap_or(false))
}

fn baseline_stale_for_close(context: &CloseTaskContext) -> CoreResult<bool> {
    let Some(basis) = context.current_close_basis.as_ref() else {
        return Ok(false);
    };
    let current_baseline = StoredScope::from_task(&context.task)?.baseline_ref;
    Ok(basis.baseline_ref.as_ref().map(BaselineRef::as_str) != current_baseline.as_deref())
}

fn recovery_required(context: &CloseTaskContext) -> CoreResult<bool> {
    if !context.blocker_refs.is_empty() {
        return Ok(true);
    }
    context
        .current_change_unit
        .as_ref()
        .map(|record| {
            let lifecycle: PersistedLifecycleState = decode_required_json(
                "change_units",
                record.change_unit_id.clone(),
                "lifecycle_json",
                Some(&record.lifecycle_json),
            )?;
            Ok(lifecycle.recovery_required)
        })
        .transpose()
        .map(|value| value.unwrap_or(false))
}

#[derive(Debug, Clone, Copy)]
struct ResidualRiskState {
    known: bool,
    visible: bool,
}

fn residual_risk_state(context: &CloseTaskContext) -> ResidualRiskState {
    let known = context
        .current_close_basis
        .as_ref()
        .map(|basis| !basis.residual_risks.is_empty())
        .unwrap_or(false);
    ResidualRiskState {
        known,
        visible: known,
    }
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
