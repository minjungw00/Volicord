use super::*;

impl CoreService {
    /// Executes `volicord.close_task` through close-readiness and terminal transition rules.
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
            if let Some(response) = reject_stale_close_write_check(
                &prepared.store,
                &prepared.context.project_state,
                &request,
            )? {
                return Ok(response);
            }
        }
        let plan_now = utc_timestamp(self.now());
        if matches!(request.intent, CloseIntent::Check | CloseIntent::Complete)
            && !request.envelope.dry_run
        {
            if let Err(error) = session_watch::run_session_watch_check(
                &prepared.store,
                &prepared.context.verified_invocation,
                Some(&request.task_id),
                &plan_now,
            ) {
                return plan_error_response(
                    &request.envelope,
                    &prepared.context.project_state,
                    PlanError::Core(error),
                );
            }
        }

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
                Some(&prepared.context.verified_invocation),
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
        let mut plan = match plan_close_task(
            &prepared.store,
            &prepared.context.project_state,
            Some(&prepared.context.verified_invocation),
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

        let continuity_plans = match plan_close_completion_continuity_records(
            self,
            &prepared.store,
            &request,
            plan.current_close_basis.as_ref(),
            prepared.context.project_state.state_version + 1,
            &plan_now,
        ) {
            Ok(records) => records,
            Err(error) => {
                return plan_error_response(
                    &request.envelope,
                    &prepared.context.project_state,
                    error,
                )
            }
        };
        if !continuity_plans.is_empty() {
            let continuity_summary = continuity_plans
                .iter()
                .map(|plan| plan.summary.clone())
                .collect::<Vec<_>>();
            plan.result_fields.insert(
                "continuity_summary".to_owned(),
                serde_json::to_value(&continuity_summary)?,
            );
            let continuity_record_ids = continuity_plans
                .iter()
                .map(|plan| plan.record_ref.record_id.as_str().to_owned())
                .collect::<Vec<_>>();
            plan.event_payload.insert(
                "continuity_record_ids".to_owned(),
                serde_json::to_value(&continuity_record_ids)?,
            );
            plan.storage_mutations
                .extend(continuity_plans.into_iter().map(|plan| plan.mutation));
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
            request.operation_category(),
            task,
            ReplayPolicy::None,
            FreshnessPolicy::None,
            MethodEffectPolicy::ReadOnly,
        )
    } else {
        mutation_method_policy(request.operation_category(), task, request.envelope.dry_run)
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

fn reject_stale_close_write_check(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
) -> CoreResult<Option<PipelineResponse>> {
    let active_write_checks = store
        .active_write_checks(&request.task_id)
        .map_err(CorePipelineError::from)?;
    Ok(active_write_checks
        .iter()
        .find(|record| record.basis_state_version != project_state.state_version)
        .map(|record| {
            stale_write_check_basis_response(&request.envelope, record, project_state.state_version)
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
    verified_invocation: Option<&VerifiedInvocationContext>,
    guarantee_profile: Option<&ProjectEnforcementProfile>,
    request: CloseTaskRequest,
    now: &UtcTimestamp,
) -> Result<CloseTaskPlan, PlanError> {
    let context = load_close_task_context(store, project_state, verified_invocation, &request)?;
    plan_close_task_with_context(
        store,
        project_state,
        verified_invocation,
        guarantee_profile,
        request,
        now,
        context,
    )
}

pub(super) fn plan_close_task_with_context(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: Option<&VerifiedInvocationContext>,
    guarantee_profile: Option<&ProjectEnforcementProfile>,
    request: CloseTaskRequest,
    now: &UtcTimestamp,
    context: CloseTaskContext,
) -> Result<CloseTaskPlan, PlanError> {
    let mut context = context;
    if context.guard_health.is_none() {
        context.guard_health =
            projected_guard_health(store, project_state, verified_invocation, &request)?;
    }
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
        blockers.extend(guard_close_blockers(project_state, &request, &context));
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

    let guarantee_display = match (verified_invocation, guarantee_profile) {
        (Some(invocation), Some(profile)) => Some(guarantee_display_from_profile(
            profile,
            invocation,
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
        write_check_summary: projected_write_check_summary(
            store,
            &request.task_id,
            response_state_version,
            *now.as_datetime(),
            guarantee_display.clone(),
        )?,
        evidence_summary: context.evidence_summary.clone(),
        close_state: Some(close_state),
        close_blockers: blockers.clone(),
        guard_health: context.guard_health.clone(),
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
        continuity_summary: Vec::new(),
        state: result_state.clone(),
        blockers: blockers.clone(),
        guard_health: context.guard_health.clone(),
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
        guard_health: context.guard_health,
    })
}

fn plan_close_completion_continuity_records(
    service: &CoreService,
    store: &CoreProjectStore,
    request: &CloseTaskRequest,
    close_basis: Option<&CurrentCloseBasis>,
    planned_state_version: u64,
    now: &UtcTimestamp,
) -> Result<Vec<PlannedProjectContinuityRecord>, PlanError> {
    if request.intent != CloseIntent::Complete {
        return Ok(Vec::new());
    }
    let Some(close_basis) = close_basis else {
        return Ok(Vec::new());
    };
    let source_change_unit_id = Some(close_basis.change_unit_id.clone());
    let continuity_context = ProjectContinuityPlanContext {
        service,
        store,
        project_id: &request.envelope.project_id,
        source_task_id: &request.task_id,
        source_change_unit_id: source_change_unit_id.as_ref(),
        planned_state_version,
        now,
    };
    let mut records = Vec::new();
    for risk in close_basis
        .residual_risks
        .iter()
        .filter(|risk| !risk.acceptance_required)
    {
        let draft = ProjectContinuityDraft {
            kind: ProjectContinuityKind::KnownLimit,
            title: format!(
                "Known limit: {}",
                short_close_continuity_title(&risk.summary)
            ),
            summary: risk.summary.clone(),
            rationale: Some(format!(
                "{} Consequence: {}",
                close_basis.result_summary, risk.consequence
            )),
            applies_to_paths: Vec::new(),
            applies_to_refs: refs_with_context(
                close_basis.result_refs.clone(),
                risk.source_refs.clone(),
            ),
            source_refs: refs_with_context(
                vec![close_basis.source_run_ref.clone()],
                risk.source_refs.clone(),
            ),
            artifact_refs: Vec::new(),
            supersedes_refs: Vec::new(),
            review_triggers: Vec::new(),
            metadata: json!({
                "source": "close_task",
                "risk_id": risk.risk_id,
                "close_basis_revision": close_basis.close_basis_revision
            }),
        };
        records.push(
            plan_project_continuity_record(continuity_context, draft).map_err(PlanError::Core)?,
        );
    }
    Ok(records)
}

fn short_close_continuity_title(value: &str) -> String {
    const MAX_CHARS: usize = 96;
    let trimmed = value.trim();
    let mut chars = trimmed.chars();
    let short = chars.by_ref().take(MAX_CHARS).collect::<String>();
    if chars.next().is_some() {
        format!("{short}...")
    } else {
        short
    }
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
    verified_invocation: Option<&VerifiedInvocationContext>,
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
        guard_health: projected_guard_health(store, project_state, verified_invocation, request)?,
        pending_user_judgment_refs,
        blocker_refs,
        evidence_summary,
        artifact_refs,
        pending_judgment_authorities: None,
        resolved_judgment_authorities: None,
    })
}

fn projected_guard_health(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: Option<&VerifiedInvocationContext>,
    request: &CloseTaskRequest,
) -> Result<Option<GuardHealthSummary>, PlanError> {
    let Some(invocation) = verified_invocation else {
        return Ok(None);
    };
    let Some(connection_id) = invocation.actor_source.agent_connection_id() else {
        return Ok(None);
    };
    let record = volicord_store::guards::guard_health_record(
        store.runtime_home(),
        request.envelope.project_id.as_str(),
        connection_id.as_str(),
    )
    .map_err(|error| {
        PlanError::Response(Box::new(store_error_response(
            &request.envelope,
            project_state,
            error,
        )))
    })?;
    let mut summary = guard_health_summary_from_record(record)?;
    if let Some(summary) = summary.as_mut() {
        summary.local_web_consent_available = invocation.local_web_consent_available;
        session_watch::apply_session_watch_status(store, invocation, summary)?;
        refresh_guard_strength(summary);
    }
    Ok(summary)
}

const REQUIRED_GUARD_HOOK_PHASES: &[&str] = &[
    "session_start_hook",
    "pre_tool_hook",
    "post_tool_hook",
    "user_prompt_submit_hook",
    "stop_hook",
];

const KNOWN_GUARD_OBSERVATION_PHASES: &[&str] = &[
    "session_start",
    "pre_tool",
    "post_tool",
    "prompt_capture",
    "stop",
];

#[derive(Debug, Clone)]
struct GuardCapabilityFacts {
    expected_policy_hash: Option<String>,
    required_hook_phases: Vec<String>,
    missing_required_hook_phases: Vec<String>,
    guard_profile: Option<String>,
    managed_bundle_hash: Option<String>,
    managed_verification_status: Option<String>,
    native_host_output_adapter_verified: bool,
    bash_shell_mutation_coverage_configured: bool,
    direct_file_write_matcher_coverage_configured: bool,
    generated_config_verified: bool,
    hook_path_safety: String,
    hook_commands_cwd_independent: bool,
    hook_commands_subdirectory_safe: bool,
}

pub(super) fn guard_health_summary_from_record(
    record: GuardHealthRecord,
) -> Result<Option<GuardHealthSummary>, PlanError> {
    let guard_mode = guard_health_mode(&record)?;
    let guard_installation_status = if let Some(installation) = record.guard_installation.as_ref() {
        parse_guard_installation_status(
            "guard_installations",
            &installation.guard_installation_id,
            &installation.installation_status,
        )?
    } else {
        GuardInstallationStatus::Absent
    };
    let capability = record
        .guard_installation
        .as_ref()
        .map(guard_capability_facts)
        .transpose()?
        .unwrap_or_else(default_guard_capability_facts);
    let guard_configuration_status =
        guard_configuration_status(guard_installation_status, &capability);
    let guard_observation_status =
        guard_observation_status(record.guard_installation.as_ref(), &capability)?;
    let effective_guard_status = effective_guard_status(
        guard_mode,
        guard_configuration_status,
        guard_observation_status,
    );
    let guard_installation_id = record
        .guard_installation
        .as_ref()
        .map(|installation| GuardInstallationId::new(installation.guard_installation_id.clone()))
        .into();
    let host_kind = record
        .guard_installation
        .as_ref()
        .map(|installation| {
            parse_owner_storage_value(
                "guard_installations",
                installation.guard_installation_id.clone(),
                "host_kind",
                &installation.host_kind,
            )
        })
        .transpose()?
        .into();
    let last_guard_event_at = record
        .latest_event
        .as_ref()
        .map(|event| {
            parse_owner_storage_value(
                "guard_events",
                event.guard_event_id.clone(),
                "occurred_at",
                &event.occurred_at,
            )
        })
        .transpose()?
        .into();
    let last_guard_observed_at: RequiredNullable<UtcTimestamp> = record
        .guard_installation
        .as_ref()
        .and_then(|installation| {
            installation.last_seen_at.as_ref().map(|last_seen_at| {
                parse_owner_storage_value(
                    "guard_installations",
                    installation.guard_installation_id.clone(),
                    "last_seen_at",
                    last_seen_at,
                )
            })
        })
        .transpose()?
        .into();
    let observed_hook_phase = record
        .guard_installation
        .as_ref()
        .and_then(|installation| installation.last_seen_phase.clone())
        .into();
    let observed_host_kind = record
        .guard_installation
        .as_ref()
        .and_then(|installation| {
            installation.observed_host_kind.as_ref().map(|host_kind| {
                parse_owner_storage_value(
                    "guard_installations",
                    installation.guard_installation_id.clone(),
                    "observed_host_kind",
                    host_kind,
                )
            })
        })
        .transpose()?
        .into();
    let guard_hook_observed = guard_observation_status == GuardObservationStatus::Observed;
    let mcp_connection_status = record
        .connection
        .as_ref()
        .map(|connection| connection.last_verification_status.clone())
        .into();
    let mcp_connection_healthy = record.connection.as_ref().is_some_and(|connection| {
        connection.enabled && connection.last_verification_status == "complete"
    });
    let prompt_capture_availability = volicord_store::guards::prompt_capture_availability(&record)
        .map_err(CorePipelineError::from)
        .map_err(PlanError::Core)?;
    let prompt_capture_status = prompt_capture_availability.status;
    let prompt_capture_available = prompt_capture_availability.can_use_chat_commands();
    let managed_distribution_verified = managed_distribution_verified(guard_mode, &capability);
    let missing_or_stale_write_readiness = record
        .latest_event
        .as_ref()
        .map(latest_guard_event_has_write_readiness_issue)
        .transpose()?
        .unwrap_or(false);
    let mut summary = GuardHealthSummary {
        guard_mode,
        guard_strength: GuardStrength::AuthorityRecordOnly,
        guard_installation_id,
        guard_installation_status,
        guard_configuration_status,
        guard_observation_status,
        effective_guard_status,
        generated_config_verified: capability.generated_config_verified,
        native_host_output_adapter_verified: capability.native_host_output_adapter_verified,
        hook_path_safety: capability.hook_path_safety,
        hook_commands_cwd_independent: capability.hook_commands_cwd_independent,
        hook_commands_subdirectory_safe: capability.hook_commands_subdirectory_safe,
        pre_tool_blocking_available: false,
        post_tool_correlation_available: false,
        bash_shell_mutation_coverage: capability.bash_shell_mutation_coverage_configured,
        direct_file_write_matcher_coverage: capability
            .direct_file_write_matcher_coverage_configured,
        bypass_detection_active: false,
        guard_hook_observed,
        last_guard_observed_at,
        last_guard_event_at,
        host_kind,
        observed_hook_phase,
        observed_host_kind,
        expected_policy_hash: capability.expected_policy_hash.into(),
        observed_policy_hash: record
            .guard_installation
            .as_ref()
            .and_then(|installation| installation.observed_policy_hash.clone())
            .into(),
        observed_binary_version: record
            .guard_installation
            .as_ref()
            .and_then(|installation| installation.observed_binary_version.clone())
            .into(),
        required_hook_phases: capability.required_hook_phases,
        missing_required_hook_phases: capability.missing_required_hook_phases,
        prompt_capture_status,
        prompt_capture_available,
        local_web_consent_available: false,
        managed_distribution_verified,
        mcp_connection_healthy,
        mcp_connection_status,
        session_watch_status: SessionWatchStatus::Disabled,
        last_session_watch_checked_at: RequiredNullable::null(),
        session_watch_baseline_created_at: RequiredNullable::null(),
        session_watch_coverage_start_at: RequiredNullable::null(),
        session_watch_coverage_basis: RequiredNullable::null(),
        session_watch_partial_coverage_warning: RequiredNullable::null(),
        session_watch_detail: RequiredNullable::null(),
        unresolved_unrecorded_change_count: record.unresolved_unrecorded_changes.len() as u64,
        missing_or_stale_write_readiness,
    };
    refresh_guard_strength(&mut summary);
    Ok(Some(summary))
}

fn default_guard_capability_facts() -> GuardCapabilityFacts {
    GuardCapabilityFacts {
        expected_policy_hash: None,
        required_hook_phases: Vec::new(),
        missing_required_hook_phases: Vec::new(),
        guard_profile: None,
        managed_bundle_hash: None,
        managed_verification_status: None,
        native_host_output_adapter_verified: false,
        bash_shell_mutation_coverage_configured: false,
        direct_file_write_matcher_coverage_configured: false,
        generated_config_verified: false,
        hook_path_safety: "not_recorded".to_owned(),
        hook_commands_cwd_independent: false,
        hook_commands_subdirectory_safe: false,
    }
}

fn guard_capability_facts(
    installation: &volicord_store::guards::GuardInstallationRecord,
) -> Result<GuardCapabilityFacts, PlanError> {
    let capability = decode_required_json_object(
        "guard_installations",
        installation.guard_installation_id.clone(),
        "host_capability_json",
        Some(&installation.host_capability_json),
    )?;
    let expected_policy_hash = capability
        .get("policy_hash")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned);
    let required_hook_phases =
        string_array_field(&capability, "required_guard_phases").unwrap_or_default();
    let missing_required_hook_phases = guard_missing_required_hook_phases(
        &required_hook_phases,
        string_array_field(&capability, "missing_required_hooks").unwrap_or_default(),
    );
    let hook_path_safety = hook_path_safety_facts(&capability, installation);
    let generated_files_verified = generated_guard_config_verified(&capability);
    Ok(GuardCapabilityFacts {
        expected_policy_hash,
        required_hook_phases,
        missing_required_hook_phases,
        guard_profile: nonempty_string_field(&capability, "guard_profile"),
        managed_bundle_hash: nonempty_string_field(&capability, "managed_bundle_hash"),
        managed_verification_status: nonempty_string_field(
            &capability,
            "managed_verification_status",
        ),
        native_host_output_adapter_verified: capability_bool_field(
            &capability,
            "native_host_output_adapter_verified",
        ),
        bash_shell_mutation_coverage_configured: capability_bool_field(
            &capability,
            "bash_shell_mutation_coverage",
        ),
        direct_file_write_matcher_coverage_configured: capability_bool_field(
            &capability,
            "direct_file_write_matcher_coverage",
        ),
        generated_config_verified: generated_files_verified && hook_path_safety.is_ok(),
        hook_path_safety: hook_path_safety.status,
        hook_commands_cwd_independent: hook_path_safety.cwd_independent,
        hook_commands_subdirectory_safe: hook_path_safety.subdirectory_safe,
    })
}

fn capability_bool_field(object: &JsonObject, field: &str) -> bool {
    object.get(field).and_then(Value::as_bool).unwrap_or(false)
}

#[derive(Debug, Clone)]
struct HookPathSafetyFacts {
    status: String,
    cwd_independent: bool,
    subdirectory_safe: bool,
}

impl HookPathSafetyFacts {
    fn ok() -> Self {
        Self {
            status: "ok".to_owned(),
            cwd_independent: true,
            subdirectory_safe: true,
        }
    }

    fn failed(status: impl Into<String>) -> Self {
        Self {
            status: status.into(),
            cwd_independent: false,
            subdirectory_safe: false,
        }
    }

    fn is_ok(&self) -> bool {
        self.status == "ok" && self.cwd_independent && self.subdirectory_safe
    }
}

fn hook_path_safety_facts(
    capability: &JsonObject,
    installation: &volicord_store::guards::GuardInstallationRecord,
) -> HookPathSafetyFacts {
    let Some(commands) = capability
        .get("host_hook_commands")
        .and_then(Value::as_array)
    else {
        return if capability_requires_hook_path_safety(capability) {
            HookPathSafetyFacts::failed("metadata_missing")
        } else {
            HookPathSafetyFacts::failed("not_recorded")
        };
    };
    if commands.is_empty() {
        return if capability_requires_hook_path_safety(capability) {
            HookPathSafetyFacts::failed("metadata_missing")
        } else {
            HookPathSafetyFacts::failed("not_recorded")
        };
    }
    let mut status = "ok";
    for command in commands {
        let command_status = recorded_hook_command_path_status(command, installation);
        if command_status != "ok" {
            status = more_severe_hook_path_status(status, command_status);
        }
    }
    if status == "ok" {
        HookPathSafetyFacts::ok()
    } else {
        HookPathSafetyFacts::failed(status)
    }
}

fn capability_requires_hook_path_safety(capability: &JsonObject) -> bool {
    matches!(
        capability.get("guard_profile").and_then(Value::as_str),
        Some("host_hook_guarded" | "managed_guarded")
    ) || capability
        .get("required_guard_phases")
        .and_then(Value::as_array)
        .is_some_and(|phases| !phases.is_empty())
}

fn recorded_hook_command_path_status(
    command: &Value,
    installation: &volicord_store::guards::GuardInstallationRecord,
) -> &'static str {
    let host_kind = command
        .get("host_kind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let phase = command
        .get("phase")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let command_text = command
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let args = command
        .get("args")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let expected_wrapper_path = command
        .get("expected_wrapper_path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let expected_phase_wrapper_path = command
        .get("expected_phase_wrapper_path")
        .and_then(Value::as_str)
        .unwrap_or(expected_wrapper_path);
    let phase_command = phase_command_name_from_capability(phase).unwrap_or_default();
    if host_kind != installation.host_kind {
        return "authority_mismatch";
    }
    if command.get("cwd_independent").and_then(Value::as_bool) != Some(true)
        || command.get("subdirectory_safe").and_then(Value::as_bool) != Some(true)
    {
        return "relative_path_unsafe";
    }
    let mut status = classify_hook_command_path(
        host_kind,
        phase_command,
        command_text,
        args,
        expected_wrapper_path,
        expected_phase_wrapper_path,
    );
    if let Some(recorded_status) = command
        .get("wrapper_resolution_status")
        .and_then(Value::as_str)
        .filter(|value| *value != "ok")
    {
        status = more_severe_hook_path_status(
            status,
            hook_path_status_from_str(recorded_status).unwrap_or("metadata_missing"),
        );
    }
    status = more_severe_hook_path_status(
        status,
        verify_hook_wrapper_path(expected_phase_wrapper_path, "wrapper_missing"),
    );
    if host_kind == "codex" {
        status = more_severe_hook_path_status(
            status,
            verify_hook_wrapper_path(expected_wrapper_path, "dispatch_missing"),
        );
    }
    status
}

fn verify_hook_wrapper_path(path_text_value: &str, missing_status: &'static str) -> &'static str {
    if path_text_value.trim().is_empty() {
        return "metadata_missing";
    }
    match std::fs::metadata(Path::new(path_text_value)) {
        Ok(metadata) if metadata.is_file() => {
            if script_is_executable_path(path_text_value) {
                "ok"
            } else {
                "wrapper_not_executable"
            }
        }
        Ok(_) => "wrapper_missing",
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => missing_status,
        Err(_) => "wrapper_missing",
    }
}

fn classify_hook_command_path(
    host_kind: &str,
    phase_command: &str,
    command_text: &str,
    args: &[Value],
    expected_wrapper_path: &str,
    expected_phase_wrapper_path: &str,
) -> &'static str {
    if phase_command.is_empty() || command_text.trim().is_empty() {
        return "metadata_missing";
    }
    match host_kind {
        "codex" => classify_codex_hook_command_path(
            phase_command,
            command_text,
            expected_wrapper_path,
            expected_phase_wrapper_path,
        ),
        "claude_code" => classify_claude_hook_command_path(
            phase_command,
            command_text,
            args,
            expected_phase_wrapper_path,
        ),
        _ => "metadata_missing",
    }
}

fn classify_codex_hook_command_path(
    phase_command: &str,
    command_text: &str,
    expected_dispatch_path: &str,
    expected_phase_wrapper_path: &str,
) -> &'static str {
    let relative_wrapper = format!(".codex/hooks/volicord-{phase_command}.sh");
    if contains_bare_relative_hook_path(command_text, ".codex/hooks/") {
        return "relative_path_unsafe";
    }
    if command_text.contains(".codex/hooks/volicord-dispatch.sh")
        || command_text.contains(&relative_wrapper)
    {
        if command_text.contains("git rev-parse --show-toplevel")
            && command_text.contains(".codex/hooks/volicord-dispatch.sh")
            && command_text.contains(phase_command)
        {
            return "ok";
        }
        if let Some(path) =
            absolute_path_ending_with(command_text, ".codex/hooks/volicord-dispatch.sh")
        {
            return if paths_equivalent_text(&path, expected_dispatch_path) {
                "ok"
            } else {
                "absolute_path_stale"
            };
        }
        if let Some(path) = absolute_path_ending_with(command_text, &relative_wrapper) {
            return if paths_equivalent_text(&path, expected_phase_wrapper_path) {
                "ok"
            } else {
                "absolute_path_stale"
            };
        }
        return "relative_path_unsafe";
    }
    if command_text.contains(&format!("volicord guard {phase_command}")) {
        return "ok";
    }
    "metadata_missing"
}

fn classify_claude_hook_command_path(
    phase_command: &str,
    command_text: &str,
    args: &[Value],
    expected_phase_wrapper_path: &str,
) -> &'static str {
    let relative_wrapper = format!(".claude/hooks/volicord-{phase_command}.sh");
    let placeholder_wrapper = format!("${{CLAUDE_PROJECT_DIR}}/{relative_wrapper}");
    if contains_bare_relative_hook_path(command_text, ".claude/hooks/") {
        return "relative_path_unsafe";
    }
    if command_text.contains("${CLAUDE_PROJECT_DIR}") {
        return if command_text == placeholder_wrapper && args.is_empty() {
            "ok"
        } else {
            "placeholder_unsupported"
        };
    }
    if command_text.contains(&relative_wrapper) {
        if let Some(path) = absolute_path_ending_with(command_text, &relative_wrapper) {
            return if paths_equivalent_text(&path, expected_phase_wrapper_path) {
                "ok"
            } else {
                "absolute_path_stale"
            };
        }
        return "relative_path_unsafe";
    }
    if command_text.contains(&format!("volicord guard {phase_command}")) {
        return "ok";
    }
    "metadata_missing"
}

fn contains_bare_relative_hook_path(command_text: &str, prefix: &str) -> bool {
    let trimmed = command_text.trim_start_matches([' ', '\'', '"']);
    trimmed.starts_with(prefix)
        || trimmed.starts_with(&format!("./{prefix}"))
        || command_text.contains(&format!(" {prefix}"))
        || command_text.contains(&format!(" './{prefix}"))
        || command_text.contains(&format!(" \"./{prefix}"))
        || command_text.contains(&format!(" '{prefix}"))
        || command_text.contains(&format!(" \"{prefix}"))
}

fn absolute_path_ending_with(command_text: &str, suffix: &str) -> Option<String> {
    let index = command_text.find(suffix)?;
    let prefix = &command_text[..index];
    let start = prefix
        .rfind([' ', '\'', '"', '=', ';', '('])
        .map(|position| position + 1)
        .unwrap_or(0);
    let path_prefix = prefix.get(start..)?;
    if !path_prefix.starts_with('/') {
        return None;
    }
    Some(format!("{path_prefix}{suffix}"))
}

fn paths_equivalent_text(left: &str, right: &str) -> bool {
    lexical_absolute_path(left)
        .is_some_and(|left| lexical_absolute_path(right).is_some_and(|right| left == right))
}

fn lexical_absolute_path(path_text_value: &str) -> Option<String> {
    let path = Path::new(path_text_value);
    if !path.is_absolute() {
        return None;
    }
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::RootDir => {}
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                parts.pop();
            }
            std::path::Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            std::path::Component::Prefix(_) => return None,
        }
    }
    Some(format!("/{}", parts.join("/")))
}

fn phase_command_name_from_capability(phase: &str) -> Option<&'static str> {
    match phase {
        "session_start_hook" | "session_start" => Some("session-start"),
        "pre_tool_hook" | "pre_tool" => Some("pre-tool"),
        "post_tool_hook" | "post_tool" => Some("post-tool"),
        "user_prompt_submit_hook" | "prompt_capture" => Some("prompt-capture"),
        "stop_hook" | "stop" => Some("stop"),
        _ => None,
    }
}

fn more_severe_hook_path_status(left: &'static str, right: &'static str) -> &'static str {
    if hook_path_status_rank(left) <= hook_path_status_rank(right) {
        left
    } else {
        right
    }
}

fn hook_path_status_rank(status: &str) -> u8 {
    match status {
        "ok" => 100,
        "metadata_missing" => 0,
        "authority_mismatch" => 1,
        "policy_hash_mismatch" => 2,
        "host_output_mismatch" => 3,
        "relative_path_unsafe" => 4,
        "absolute_path_stale" => 5,
        "placeholder_unsupported" => 6,
        "dispatch_missing" => 7,
        "wrapper_missing" => 8,
        "wrapper_not_executable" => 9,
        _ => 10,
    }
}

fn hook_path_status_from_str(status: &str) -> Option<&'static str> {
    match status {
        "ok" => Some("ok"),
        "metadata_missing" => Some("metadata_missing"),
        "authority_mismatch" => Some("authority_mismatch"),
        "policy_hash_mismatch" => Some("policy_hash_mismatch"),
        "host_output_mismatch" => Some("host_output_mismatch"),
        "relative_path_unsafe" => Some("relative_path_unsafe"),
        "absolute_path_stale" => Some("absolute_path_stale"),
        "placeholder_unsupported" => Some("placeholder_unsupported"),
        "dispatch_missing" => Some("dispatch_missing"),
        "wrapper_missing" => Some("wrapper_missing"),
        "wrapper_not_executable" => Some("wrapper_not_executable"),
        _ => None,
    }
}

fn generated_guard_config_verified(capability: &JsonObject) -> bool {
    let Some(files) = capability.get("files").and_then(Value::as_array) else {
        return false;
    };
    if files.is_empty() {
        return false;
    }
    let mut has_policy_file = false;
    let mut has_hook_config = false;
    let mut has_hook_wrapper = false;
    for file in files {
        let Some(kind) = file.get("kind").and_then(Value::as_str) else {
            return false;
        };
        match kind {
            "volicord_policy" => has_policy_file = true,
            "host_hook_config" => has_hook_config = true,
            "host_hook_wrapper" => has_hook_wrapper = true,
            _ => {}
        }
        if !generated_guard_file_verified(file) {
            return false;
        }
    }
    has_policy_file && has_hook_config && has_hook_wrapper
}

fn generated_guard_file_verified(file: &Value) -> bool {
    let Some(path_text) = file.get("path").and_then(Value::as_str) else {
        return false;
    };
    let Ok(text) = std::fs::read_to_string(Path::new(path_text)) else {
        return false;
    };
    let expected_hash = file
        .get("content_hash")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match file.get("ownership").and_then(Value::as_str) {
        Some("managed_block") => generated_managed_block_verified(file, &text, expected_hash),
        Some("managed_json") => sha256_text(&text) == expected_hash,
        Some("managed_json_projection") => {
            generated_json_projection_verified(file, &text, expected_hash)
        }
        Some("managed_script") => generated_script_verified(file, &text, expected_hash),
        _ => false,
    }
}

fn generated_managed_block_verified(file: &Value, text: &str, expected_hash: &str) -> bool {
    let Some(start_marker) = file.get("managed_marker_start").and_then(Value::as_str) else {
        return false;
    };
    let Some(end_marker) = file.get("managed_marker_end").and_then(Value::as_str) else {
        return false;
    };
    if marker_count(text, start_marker) != 1 || marker_count(text, end_marker) != 1 {
        return false;
    }
    let Some(block) = managed_block_slice(text, start_marker, end_marker) else {
        return false;
    };
    sha256_text(block) == expected_hash
}

fn generated_json_projection_verified(file: &Value, text: &str, expected_hash: &str) -> bool {
    let Some(expected_projection_json) =
        file.get("managed_projection_json").and_then(Value::as_str)
    else {
        return false;
    };
    if sha256_text(expected_projection_json) != expected_hash {
        return false;
    }
    let Ok(actual) = serde_json::from_str::<Value>(text) else {
        return false;
    };
    let Ok(desired) = serde_json::from_str::<Value>(expected_projection_json) else {
        return false;
    };
    managed_projection_present(&actual, &desired)
}

fn generated_script_verified(file: &Value, text: &str, expected_hash: &str) -> bool {
    let Some(managed_marker) = file.get("managed_marker").and_then(Value::as_str) else {
        return false;
    };
    if !text.contains(managed_marker) {
        return false;
    }
    if sha256_text(text) != expected_hash {
        return false;
    }
    let Some(expected_command) = file
        .get("managed_script_command")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    else {
        return false;
    };
    if hook_wrapper_exec_command(text) != Some(expected_command) {
        return false;
    }
    for key in [
        "host_kind",
        "phase",
        "connection_id",
        "guard_installation_id",
        "policy_hash",
        "host_output",
    ] {
        let Some(expected) = file.get(key).and_then(Value::as_str) else {
            return false;
        };
        if hook_wrapper_comment_value(text, key) != Some(expected) {
            return false;
        }
    }
    if file
        .get("executable_required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && !script_is_executable(file)
    {
        return false;
    }
    true
}

fn marker_count(text: &str, marker: &str) -> usize {
    text.match_indices(marker).count()
}

fn managed_block_slice<'a>(text: &'a str, start_marker: &str, end_marker: &str) -> Option<&'a str> {
    let start = text.find(start_marker)?;
    let end = start + text[start..].find(end_marker)? + end_marker.len();
    let end = if text[end..].starts_with('\n') {
        end + 1
    } else {
        end
    };
    text.get(start..end)
}

fn managed_projection_present(actual: &Value, desired: &Value) -> bool {
    let Some(desired_object) = desired.as_object() else {
        return actual == desired;
    };
    desired_object.iter().all(|(key, desired_value)| {
        let Some(actual_value) = actual.get(key) else {
            return false;
        };
        if key == "hooks" || key == "mcpServers" {
            return managed_projection_object_present(actual_value, desired_value);
        }
        managed_projection_present(actual_value, desired_value)
    })
}

fn managed_projection_object_present(actual: &Value, desired: &Value) -> bool {
    let (Some(actual_object), Some(desired_object)) = (actual.as_object(), desired.as_object())
    else {
        return false;
    };
    desired_object.iter().all(|(key, desired_value)| {
        let Some(actual_value) = actual_object.get(key) else {
            return false;
        };
        match (actual_value.as_array(), desired_value.as_array()) {
            (Some(actual_array), Some(desired_array)) => desired_array.iter().all(|desired_item| {
                let desired_count = desired_array
                    .iter()
                    .filter(|item| *item == desired_item)
                    .count();
                let actual_count = actual_array
                    .iter()
                    .filter(|item| *item == desired_item)
                    .count();
                actual_count == desired_count
            }),
            _ => actual_value == desired_value,
        }
    })
}

fn hook_wrapper_exec_command(text: &str) -> Option<&str> {
    text.lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("exec "))
}

fn hook_wrapper_comment_value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("# {key}=");
    text.lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix(&prefix))
        .map(str::trim)
}

#[cfg(unix)]
fn script_is_executable(file: &Value) -> bool {
    use std::os::unix::fs::PermissionsExt;

    let Some(path_text) = file.get("path").and_then(Value::as_str) else {
        return false;
    };
    std::fs::metadata(Path::new(path_text))
        .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(unix)]
fn script_is_executable_path(path_text: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;

    std::fs::metadata(Path::new(path_text))
        .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn script_is_executable(_file: &Value) -> bool {
    true
}

#[cfg(not(unix))]
fn script_is_executable_path(_path_text: &str) -> bool {
    true
}

fn sha256_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("sha256:{}", hex_bytes(&hasher.finalize()))
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn nonempty_string_field(object: &JsonObject, field: &str) -> Option<String> {
    object
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
}

fn string_array_field(object: &JsonObject, field: &str) -> Option<Vec<String>> {
    Some(
        object
            .get(field)?
            .as_array()?
            .iter()
            .filter_map(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned)
            .collect(),
    )
}

fn managed_distribution_verified(guard_mode: GuardMode, capability: &GuardCapabilityFacts) -> bool {
    guard_mode == GuardMode::Managed
        && capability.guard_profile.as_deref() == Some("managed_guarded")
        && capability.managed_verification_status.as_deref() == Some("verified")
        && capability.managed_bundle_hash.is_some()
}

pub(super) fn refresh_guard_strength(summary: &mut GuardHealthSummary) {
    let host_hook_strength_available = host_hook_strength_available(summary);
    summary.pre_tool_blocking_available =
        host_hook_strength_available && required_hook_available(summary, "pre_tool_hook");
    summary.post_tool_correlation_available =
        host_hook_strength_available && required_hook_available(summary, "post_tool_hook");
    summary.bypass_detection_active = summary.session_watch_status == SessionWatchStatus::Active;
    summary.guard_strength =
        if summary.managed_distribution_verified && host_hook_strength_available {
            GuardStrength::ManagedGuarded
        } else if host_hook_strength_available {
            GuardStrength::HostHookGuarded
        } else if summary.bypass_detection_active {
            GuardStrength::DetectiveWatch
        } else {
            GuardStrength::AuthorityRecordOnly
        };
}

fn host_hook_strength_available(summary: &GuardHealthSummary) -> bool {
    matches!(summary.guard_mode, GuardMode::Guarded | GuardMode::Managed)
        && summary.effective_guard_status == GuardEffectiveStatus::Active
        && summary.guard_configuration_status == GuardConfigurationStatus::Configured
        && summary.guard_observation_status == GuardObservationStatus::Observed
        && summary.guard_hook_observed
        && summary.generated_config_verified
        && summary.native_host_output_adapter_verified
        && summary.hook_path_safety == "ok"
        && summary.hook_commands_cwd_independent
        && summary.hook_commands_subdirectory_safe
        && summary
            .expected_policy_hash
            .as_ref()
            .is_some_and(|expected| {
                summary
                    .observed_policy_hash
                    .as_ref()
                    .is_some_and(|observed| observed == expected)
            })
        && summary.observed_host_kind.as_ref() == summary.host_kind.as_ref()
        && REQUIRED_GUARD_HOOK_PHASES
            .iter()
            .all(|phase| required_hook_available(summary, phase))
        && summary
            .required_hook_phases
            .iter()
            .any(|phase| phase == "pre_tool_hook")
        && summary
            .required_hook_phases
            .iter()
            .any(|phase| phase == "post_tool_hook")
        && summary
            .required_hook_phases
            .iter()
            .any(|phase| phase == "stop_hook")
        && (!summary.prompt_capture_available
            || summary
                .required_hook_phases
                .iter()
                .any(|phase| phase == "user_prompt_submit_hook"))
        && summary.bash_shell_mutation_coverage
        && summary.direct_file_write_matcher_coverage
}

fn required_hook_available(summary: &GuardHealthSummary, phase: &str) -> bool {
    summary.effective_guard_status == GuardEffectiveStatus::Active
        && summary
            .required_hook_phases
            .iter()
            .any(|configured| configured == phase)
        && !summary
            .missing_required_hook_phases
            .iter()
            .any(|missing| missing == phase)
}

fn guard_missing_required_hook_phases(
    configured_required_hook_phases: &[String],
    mut explicit_missing_hook_phases: Vec<String>,
) -> Vec<String> {
    for required_phase in REQUIRED_GUARD_HOOK_PHASES {
        if !configured_required_hook_phases
            .iter()
            .any(|phase| phase == required_phase)
        {
            explicit_missing_hook_phases.push((*required_phase).to_owned());
        }
    }
    explicit_missing_hook_phases.sort();
    explicit_missing_hook_phases.dedup();
    explicit_missing_hook_phases
}

fn guard_configuration_status(
    installation_status: GuardInstallationStatus,
    capability: &GuardCapabilityFacts,
) -> GuardConfigurationStatus {
    if !capability.missing_required_hook_phases.is_empty()
        && !matches!(
            installation_status,
            GuardInstallationStatus::Absent
                | GuardInstallationStatus::Stale
                | GuardInstallationStatus::Broken
        )
    {
        return GuardConfigurationStatus::Degraded;
    }
    match installation_status {
        GuardInstallationStatus::Absent => GuardConfigurationStatus::Absent,
        GuardInstallationStatus::Configured | GuardInstallationStatus::Active => {
            GuardConfigurationStatus::Configured
        }
        GuardInstallationStatus::ReloadRequired => GuardConfigurationStatus::ReloadRequired,
        GuardInstallationStatus::Degraded => GuardConfigurationStatus::Degraded,
        GuardInstallationStatus::Stale => GuardConfigurationStatus::Stale,
        GuardInstallationStatus::Broken => GuardConfigurationStatus::Broken,
    }
}

fn guard_observation_status(
    installation: Option<&volicord_store::guards::GuardInstallationRecord>,
    capability: &GuardCapabilityFacts,
) -> Result<GuardObservationStatus, PlanError> {
    let Some(installation) = installation else {
        return Ok(GuardObservationStatus::NotObserved);
    };
    let Some(last_seen_at) = installation.last_seen_at.as_deref() else {
        return Ok(GuardObservationStatus::NotObserved);
    };
    parse_owner_storage_value::<UtcTimestamp>(
        "guard_installations",
        installation.guard_installation_id.clone(),
        "last_seen_at",
        last_seen_at,
    )?;
    let current_host_kind =
        installation.observed_host_kind.as_deref() == Some(installation.host_kind.as_str());
    let current_policy_hash = capability
        .expected_policy_hash
        .as_deref()
        .is_some_and(|expected| installation.observed_policy_hash.as_deref() == Some(expected));
    let known_phase = installation
        .last_seen_phase
        .as_deref()
        .is_some_and(|phase| KNOWN_GUARD_OBSERVATION_PHASES.contains(&phase));
    if current_host_kind && current_policy_hash && known_phase {
        Ok(GuardObservationStatus::Observed)
    } else {
        Ok(GuardObservationStatus::StaleObservation)
    }
}

fn effective_guard_status(
    guard_mode: GuardMode,
    configuration_status: GuardConfigurationStatus,
    observation_status: GuardObservationStatus,
) -> GuardEffectiveStatus {
    if guard_mode == GuardMode::McpOnly {
        return GuardEffectiveStatus::Inactive;
    }
    match configuration_status {
        GuardConfigurationStatus::Absent => GuardEffectiveStatus::Inactive,
        GuardConfigurationStatus::Broken => GuardEffectiveStatus::Broken,
        GuardConfigurationStatus::Stale | GuardConfigurationStatus::Degraded => {
            GuardEffectiveStatus::Degraded
        }
        GuardConfigurationStatus::ReloadRequired => GuardEffectiveStatus::ActionRequired,
        GuardConfigurationStatus::Configured => {
            if observation_status == GuardObservationStatus::Observed {
                GuardEffectiveStatus::Active
            } else {
                GuardEffectiveStatus::ActionRequired
            }
        }
    }
}

fn guard_health_mode(record: &GuardHealthRecord) -> Result<GuardMode, PlanError> {
    if let Some(installation) = record.guard_installation.as_ref() {
        return parse_guard_mode(
            "guard_installations",
            &installation.guard_installation_id,
            &installation.guard_mode,
        );
    }
    if let Some(session) = record.latest_session.as_ref() {
        return parse_guard_mode("agent_sessions", &session.session_id, &session.guard_mode);
    }
    Ok(GuardMode::McpOnly)
}

fn parse_guard_mode(
    table: &'static str,
    record_ref: &str,
    value: &str,
) -> Result<GuardMode, PlanError> {
    serde_json::from_value(Value::String(value.to_owned()))
        .map_err(|_| {
            CorePipelineError::Store(StoreError::corrupt_owner_state_value(
                table,
                record_ref.to_owned(),
                "guard_mode",
            ))
        })
        .map_err(PlanError::Core)
}

fn parse_guard_installation_status(
    table: &'static str,
    record_ref: &str,
    value: &str,
) -> Result<GuardInstallationStatus, PlanError> {
    serde_json::from_value(Value::String(value.to_owned()))
        .map_err(|_| {
            CorePipelineError::Store(StoreError::corrupt_owner_state_value(
                table,
                record_ref.to_owned(),
                "installation_status",
            ))
        })
        .map_err(PlanError::Core)
}

fn latest_guard_event_has_write_readiness_issue(
    event: &volicord_store::guards::GuardEventRecord,
) -> Result<bool, PlanError> {
    let result = decode_required_json_object(
        "guard_events",
        event.guard_event_id.clone(),
        "result_json",
        Some(&event.result_json),
    )?;
    let result = Value::Object(result);
    Ok(
        json_has_code(&result, &["write_readiness_missing", "write_check_stale"])
            || json_has_non_empty_array_key(&result, "stale_write_check_ids"),
    )
}

fn json_has_code(value: &Value, codes: &[&str]) -> bool {
    match value {
        Value::Object(object) => {
            object
                .get("code")
                .and_then(Value::as_str)
                .is_some_and(|code| codes.contains(&code))
                || object.values().any(|value| json_has_code(value, codes))
        }
        Value::Array(values) => values.iter().any(|value| json_has_code(value, codes)),
        _ => false,
    }
}

fn json_has_non_empty_array_key(value: &Value, key: &str) -> bool {
    match value {
        Value::Object(object) => {
            object
                .get(key)
                .and_then(Value::as_array)
                .is_some_and(|values| !values.is_empty())
                || object
                    .values()
                    .any(|value| json_has_non_empty_array_key(value, key))
        }
        Value::Array(values) => values
            .iter()
            .any(|value| json_has_non_empty_array_key(value, key)),
        _ => false,
    }
}

fn guard_close_blockers(
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    context: &CloseTaskContext,
) -> Vec<CloseReadinessBlocker> {
    let Some(summary) = context.guard_health.as_ref() else {
        return Vec::new();
    };
    let mcp_only_watch_blocks = summary.guard_mode == GuardMode::McpOnly
        && summary.session_watch_status == SessionWatchStatus::Active
        && summary.unresolved_unrecorded_change_count > 0;
    if !matches!(summary.guard_mode, GuardMode::Guarded | GuardMode::Managed)
        && !mcp_only_watch_blocks
    {
        return Vec::new();
    }

    let task_ref = task_ref_for_close(request, project_state.state_version);
    let mut blockers = Vec::new();
    if summary.guard_mode == GuardMode::McpOnly {
        if summary.unresolved_unrecorded_change_count > 0 {
            let can_resolve_in_chat = user_channel_can_resolve_in_chat(Some(summary));
            blockers.push(close_blocker_with_resolution(
                CloseReadinessBlockerCategory::ConnectionCapability,
                "unresolved_unrecorded_changes",
                "Observed Product Repository changes still need reconciliation.",
                can_resolve_in_chat,
                !can_resolve_in_chat,
                vec![task_ref.clone()],
                vec![NextActionSummary {
                    action_kind: NextActionKind::ReconcileChanges,
                    owner_method: Some(MethodName::ReconcileChanges),
                    label:
                        "Run reconciliation for observed Product Repository changes before close."
                            .to_owned(),
                    blocking_question: Some(
                        "Does the user accept any remaining observed Product Repository change as intentional?"
                            .to_owned(),
                    ),
                    required_refs: vec![task_ref],
                }],
            ));
        }
        return guard_blockers_with_strength(blockers, summary.guard_strength);
    }
    if let Some(blocker) = guard_installation_close_blocker(summary, &task_ref) {
        blockers.push(blocker);
    }
    if summary.guard_mode == GuardMode::Managed
        && (summary.session_watch_status != SessionWatchStatus::Active
            || summary.session_watch_partial_coverage_warning.is_some())
    {
        let message = if summary.session_watch_status == SessionWatchStatus::Active {
            "Managed close requires full Product Repository session-watch coverage."
        } else {
            "Managed close requires an active Product Repository session watch."
        };
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::ConnectionCapability,
            "session_watch_unavailable",
            message,
            vec![task_ref.clone()],
            vec![NextActionSummary {
                action_kind: NextActionKind::CloseTask,
                owner_method: None,
                label: "Repair or retry session watch before completing the Task.".to_owned(),
                blocking_question: None,
                required_refs: vec![task_ref.clone()],
            }],
        ));
    }
    if !summary.mcp_connection_healthy {
        blockers.push(close_blocker_with_resolution(
            CloseReadinessBlockerCategory::ConnectionCapability,
            "guard_connection_unhealthy",
            "Guarded close requires the Agent Connection to be healthy.",
            false,
            true,
            vec![task_ref.clone()],
            vec![NextActionSummary {
                action_kind: NextActionKind::CloseTask,
                owner_method: None,
                label: "Repair Agent Connection health before completing the Task.".to_owned(),
                blocking_question: None,
                required_refs: vec![task_ref.clone()],
            }],
        ));
    }
    if summary.unresolved_unrecorded_change_count > 0 {
        let can_resolve_in_chat = user_channel_can_resolve_in_chat(Some(summary));
        blockers.push(close_blocker_with_resolution(
            CloseReadinessBlockerCategory::ConnectionCapability,
            "unresolved_unrecorded_changes",
            "Observed Product Repository changes still need reconciliation.",
            can_resolve_in_chat,
            !can_resolve_in_chat,
            vec![task_ref.clone()],
            vec![NextActionSummary {
                action_kind: NextActionKind::ReconcileChanges,
                owner_method: Some(MethodName::ReconcileChanges),
                label: "Run reconciliation for observed Product Repository changes before close."
                    .to_owned(),
                blocking_question: Some(
                    "Does the user accept any remaining observed Product Repository change as intentional?"
                        .to_owned(),
                ),
                required_refs: vec![task_ref.clone()],
            }],
        ));
    }
    if summary.missing_or_stale_write_readiness {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::WriteCompatibility,
            "guard_write_readiness_missing_or_stale",
            "Guard events detected missing or stale write readiness.",
            vec![task_ref.clone()],
            vec![NextActionSummary {
                action_kind: NextActionKind::PrepareWrite,
                owner_method: Some(MethodName::PrepareWrite),
                label: "Refresh write readiness before completing the Task.".to_owned(),
                blocking_question: None,
                required_refs: vec![task_ref],
            }],
        ));
    }
    guard_blockers_with_strength(blockers, summary.guard_strength)
}

fn guard_blockers_with_strength(
    mut blockers: Vec<CloseReadinessBlocker>,
    guard_strength: GuardStrength,
) -> Vec<CloseReadinessBlocker> {
    for blocker in &mut blockers {
        blocker.guard_strength = Some(guard_strength);
    }
    blockers
}

pub(super) fn user_channel_pending_judgment_instruction(
    guard_health: Option<&GuardHealthSummary>,
) -> String {
    if guard_health.is_some_and(|summary| summary.mcp_connection_healthy) {
        "Use MCP elicitation for the pending user-owned judgment.".to_owned()
    } else if guard_health.is_some_and(|summary| summary.prompt_capture_available) {
        "Use the displayed prompt-capture chat command with the current verification code."
            .to_owned()
    } else if guard_health.is_some_and(|summary| summary.local_web_consent_available) {
        "Use the local web consent fallback if the adapter offers a loopback consent link."
            .to_owned()
    } else {
        "Use the local volicord user command as the recovery path.".to_owned()
    }
}

pub(super) fn user_channel_can_resolve_in_chat(guard_health: Option<&GuardHealthSummary>) -> bool {
    guard_health
        .map(|summary| summary.mcp_connection_healthy || summary.prompt_capture_available)
        .unwrap_or(false)
}

fn guard_installation_close_blocker(
    summary: &GuardHealthSummary,
    task_ref: &StateRecordRef,
) -> Option<CloseReadinessBlocker> {
    if summary.effective_guard_status == GuardEffectiveStatus::Active {
        return None;
    }
    let host_kind = summary
        .host_kind
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| "unknown".to_owned());
    let missing_phases = if summary.missing_required_hook_phases.is_empty() {
        "none".to_owned()
    } else {
        summary.missing_required_hook_phases.join(", ")
    };
    let observation_detail = format!(
        "host_kind={host_kind}; observation_status={}; missing_required_hooks={missing_phases}",
        summary.guard_observation_status.as_str()
    );
    let (code, message, label) = match summary.guard_configuration_status {
        GuardConfigurationStatus::Absent => (
            "guard_not_installed",
            format!("Guarded close requires a recorded guard installation ({observation_detail})."),
            format!("Install the guard integration for host {host_kind} before completing the Task."),
        ),
        GuardConfigurationStatus::ReloadRequired => (
            "guard_reload_required",
            format!("Guard files are installed, but the host has not reloaded them ({observation_detail})."),
            format!("Restart or reload host {host_kind} so it loads the Volicord guard hooks."),
        ),
        GuardConfigurationStatus::Configured
            if summary.guard_observation_status == GuardObservationStatus::StaleObservation =>
        {
            (
                "guard_not_observed",
                format!("Guard files are configured, but the latest observation does not match the current installation ({observation_detail})."),
                format!("Run a current guard hook for host {host_kind} before completing the Task."),
            )
        }
        GuardConfigurationStatus::Configured => (
            "guard_not_observed",
            format!("Guard files are configured, but no matching guard hook has been observed ({observation_detail})."),
            format!("Start or reload host {host_kind} and let the Volicord guard hook run before close."),
        ),
        GuardConfigurationStatus::Stale => (
            "guard_stale",
            format!("Guard health is stale for this guarded close path ({observation_detail})."),
            format!("Refresh or reinstall the guard integration for host {host_kind} before completing the Task."),
        ),
        GuardConfigurationStatus::Broken => (
            "guard_broken",
            format!("Guard health is broken for this guarded close path ({observation_detail})."),
            format!("Repair the guard integration for host {host_kind} before completing the Task."),
        ),
        GuardConfigurationStatus::Degraded if !summary.missing_required_hook_phases.is_empty() => (
            "guard_required_hooks_missing",
            format!("Guard configuration is missing required hook phases for this guarded close path ({observation_detail})."),
            format!("Install required guard hook phases for host {host_kind}: {missing_phases}."),
        ),
        GuardConfigurationStatus::Degraded if guard_degraded_blocks_close(summary.guard_mode) => (
            "guard_degraded",
            format!("Guard health is degraded and the current guard policy blocks close ({observation_detail})."),
            format!("Repair degraded guard health for host {host_kind} before completing the Task."),
        ),
        GuardConfigurationStatus::Degraded => return None,
    };
    Some(close_blocker_with_resolution(
        CloseReadinessBlockerCategory::ConnectionCapability,
        code,
        message,
        false,
        true,
        vec![task_ref.clone()],
        vec![NextActionSummary {
            action_kind: NextActionKind::CloseTask,
            owner_method: None,
            label,
            blocking_question: None,
            required_refs: vec![task_ref.clone()],
        }],
    ))
}

fn guard_degraded_blocks_close(guard_mode: GuardMode) -> bool {
    matches!(guard_mode, GuardMode::Guarded | GuardMode::Managed)
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
                    CloseReadinessBlockerCategory::PendingUserJudgment,
                    "pending_user_judgment",
                    "A user-owned judgment required before superseding this Task is still pending.",
                    pending_refs.clone(),
                    vec![NextActionSummary {
                        action_kind: NextActionKind::RecordUserJudgment,
                        owner_method: Some(MethodName::RecordUserJudgment),
                        label: "Resolve pending user-owned judgments through the User Channel."
                            .to_owned(),
                        blocking_question: Some(user_channel_pending_judgment_instruction(
                            context.guard_health.as_ref(),
                        )),
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
            && authority.resolution.as_ref().is_some_and(|resolution| {
                resolution.resolved_by_actor_source == ActorSource::LocalUser
            })
            && verified_user_channel_provenance(authority)
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
            CloseReadinessBlockerCategory::PendingUserJudgment,
            "pending_user_judgment",
            "A user-owned judgment required before close is still pending.",
            close_complete_pending_refs.clone(),
            vec![NextActionSummary {
                action_kind: NextActionKind::RecordUserJudgment,
                owner_method: Some(MethodName::RecordUserJudgment),
                label: "Resolve pending user-owned judgments through the User Channel.".to_owned(),
                blocking_question: Some(user_channel_pending_judgment_instruction(
                    context.guard_health.as_ref(),
                )),
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
        .active_write_checks(&request.task_id)
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
            "write_check_stale",
            "An active Write Check is stale against the current state version.",
            vec![write_check_ref(record, project_state.state_version)],
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

    blockers.extend(close_evidence_blockers(
        store,
        project_state,
        request,
        context,
        change_unit_ref.clone(),
    )?);

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

    if let Some(blocker) = final_acceptance_blocker(store, project_state, request, context)? {
        blockers.push(blocker);
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
        let stale_refs = non_current_judgment_refs_for_plan(
            store,
            project_state,
            request,
            JudgmentKind::ResidualRiskAcceptance,
        )?;
        let (code, message) = if stale_refs.is_empty() {
            (
                "missing_residual_risk_acceptance",
                "Visible residual risk requires distinct residual-risk acceptance.",
            )
        } else {
            (
                "stale_residual_risk_acceptance",
                "The available residual-risk acceptance is stale or incompatible with the current close basis.",
            )
        };
        let related_refs = refs_with_context(vec![task_ref.clone()], stale_refs);
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::ResidualRiskAcceptance,
            code,
            message,
            related_refs,
            vec![NextActionSummary {
                action_kind: NextActionKind::RequestUserJudgment,
                owner_method: Some(MethodName::RequestUserJudgment),
                label: "Request current residual-risk acceptance from the user.".to_owned(),
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
                provenance: None,
                supporting_refs: Vec::new(),
                observation_refs: Vec::new(),
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
    let observation_refs = unique_state_record_refs(
        coverage_items
            .iter()
            .flat_map(|item| item.observation_refs.clone())
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
        observation_refs,
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
        created_by_actor_source: artifact_ref.created_by_actor_source.clone(),
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
            "Completion requires a current close basis recorded by volicord.record_run.",
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
            !run_record_matches_close_basis_context(
                record,
                &request.envelope.project_id,
                &request.task_id,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CloseEvidenceIssueKind {
    Missing,
    Unsupported,
    Stale,
    AgentReportOnly,
    InsufficientProvenance,
}

struct CloseEvidenceIssue {
    kind: CloseEvidenceIssueKind,
    related_refs: Vec<StateRecordRef>,
}

fn close_evidence_blockers(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    context: &CloseTaskContext,
    change_unit_ref: Option<StateRecordRef>,
) -> Result<Vec<CloseReadinessBlocker>, PlanError> {
    let Some(summary) = context.evidence_summary.as_ref() else {
        return Ok(Vec::new());
    };
    let mut grouped: BTreeMap<CloseEvidenceIssueKind, Vec<StateRecordRef>> = BTreeMap::new();
    for item in &summary.coverage_items {
        if let Some(issue) =
            close_evidence_issue_for_item(store, project_state, request, context, item)?
        {
            grouped
                .entry(issue.kind)
                .or_default()
                .extend(issue.related_refs);
        }
    }

    let required_refs = change_unit_ref.into_iter().collect::<Vec<_>>();
    let mut blockers = Vec::new();
    for kind in [
        CloseEvidenceIssueKind::Missing,
        CloseEvidenceIssueKind::Unsupported,
        CloseEvidenceIssueKind::Stale,
        CloseEvidenceIssueKind::AgentReportOnly,
        CloseEvidenceIssueKind::InsufficientProvenance,
    ] {
        let Some(related_refs) = grouped.remove(&kind) else {
            continue;
        };
        let category = match kind {
            CloseEvidenceIssueKind::Missing | CloseEvidenceIssueKind::Unsupported => {
                CloseReadinessBlockerCategory::EvidenceClaim
            }
            CloseEvidenceIssueKind::Stale
            | CloseEvidenceIssueKind::AgentReportOnly
            | CloseEvidenceIssueKind::InsufficientProvenance => {
                CloseReadinessBlockerCategory::EvidenceProvenance
            }
        };
        let (code, message) = match kind {
            CloseEvidenceIssueKind::Missing => (
                "evidence_claim_missing",
                "One or more required close evidence claims are missing.",
            ),
            CloseEvidenceIssueKind::Unsupported => (
                "evidence_claim_unsupported",
                "One or more required close evidence claims are unsupported.",
            ),
            CloseEvidenceIssueKind::Stale => (
                "evidence_provenance_stale",
                "Evidence provenance exists but is stale against the current close basis.",
            ),
            CloseEvidenceIssueKind::AgentReportOnly => (
                "evidence_agent_report_only",
                "Required close evidence is supported only by cooperative agent reports.",
            ),
            CloseEvidenceIssueKind::InsufficientProvenance => (
                "evidence_provenance_insufficient",
                "Required close evidence lacks sufficient source provenance.",
            ),
        };
        blockers.push(close_blocker(
            category,
            code,
            message,
            unique_state_record_refs(related_refs),
            vec![NextActionSummary {
                action_kind: NextActionKind::RecordRun,
                owner_method: Some(MethodName::RecordRun),
                label: "Record evidence that supports the required close claims.".to_owned(),
                blocking_question: None,
                required_refs: required_refs.clone(),
            }],
        ));
    }
    Ok(blockers)
}

fn close_evidence_issue_for_item(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    context: &CloseTaskContext,
    item: &EvidenceCoverageItem,
) -> Result<Option<CloseEvidenceIssue>, PlanError> {
    if !item.required_for_close || item.coverage_state == EvidenceCoverageState::NotApplicable {
        return Ok(None);
    }
    if item.coverage_state != EvidenceCoverageState::Supported {
        let kind = if item.coverage_state == EvidenceCoverageState::Stale {
            CloseEvidenceIssueKind::Stale
        } else if evidence_item_has_no_support(item) {
            CloseEvidenceIssueKind::Missing
        } else {
            CloseEvidenceIssueKind::Unsupported
        };
        return Ok(Some(CloseEvidenceIssue {
            kind,
            related_refs: evidence_item_related_refs(item),
        }));
    }

    let Some(basis) = context.current_close_basis.as_ref() else {
        return Ok(Some(CloseEvidenceIssue {
            kind: CloseEvidenceIssueKind::Missing,
            related_refs: evidence_item_related_refs(item),
        }));
    };
    if item.observation_refs.is_empty() {
        return Ok(Some(CloseEvidenceIssue {
            kind: CloseEvidenceIssueKind::InsufficientProvenance,
            related_refs: evidence_item_related_refs(item),
        }));
    }

    let mut has_stale = false;
    let mut has_current_cooperative_agent_report = false;
    let mut has_current_weak = false;
    let evidence_state_version = basis
        .evidence_summary_ref
        .as_ref()
        .and_then(|record_ref| record_ref.state_version.as_ref().copied());
    for observation_ref in &item.observation_refs {
        if observation_ref.record_kind != StateRecordKind::EvidenceObservation
            || observation_ref.project_id != request.envelope.project_id
            || observation_ref.task_id.as_ref() != Some(&request.task_id)
        {
            has_current_weak = true;
            continue;
        }
        if evidence_state_version.is_some_and(|state_version| {
            observation_ref.state_version.as_ref() != Some(&state_version)
        }) {
            has_stale = true;
            continue;
        }
        let record = store
            .evidence_observation_record(observation_ref.record_id.as_str())
            .map_err(|error| {
                PlanError::Response(Box::new(store_error_response(
                    &request.envelope,
                    project_state,
                    error,
                )))
            })?;
        let Some(record) = record else {
            has_current_weak = true;
            continue;
        };
        if evidence_observation_is_stale_for_close_basis(&record, request, basis, item) {
            has_stale = true;
            continue;
        }
        match evidence_observation_provenance_class(&record)? {
            EvidenceProvenanceClass::Strong => return Ok(None),
            EvidenceProvenanceClass::CooperativeAgentReport => {
                has_current_cooperative_agent_report = true;
            }
            EvidenceProvenanceClass::Weak => {
                has_current_weak = true;
            }
        }
    }

    let kind = if has_current_cooperative_agent_report && !has_current_weak {
        CloseEvidenceIssueKind::AgentReportOnly
    } else if has_stale && !has_current_cooperative_agent_report && !has_current_weak {
        CloseEvidenceIssueKind::Stale
    } else {
        CloseEvidenceIssueKind::InsufficientProvenance
    };
    Ok(Some(CloseEvidenceIssue {
        kind,
        related_refs: evidence_item_related_refs(item),
    }))
}

fn evidence_observation_is_stale_for_close_basis(
    record: &EvidenceObservationRecord,
    request: &CloseTaskRequest,
    basis: &CurrentCloseBasis,
    item: &EvidenceCoverageItem,
) -> bool {
    record.project_id != request.envelope.project_id.as_str()
        || record.task_id != request.task_id.as_str()
        || record.change_unit_id.as_deref() != Some(basis.change_unit_id.as_str())
        || record.run_id.as_deref() != Some(basis.source_run_ref.record_id.as_str())
        || record.claim.trim() != item.claim
}

fn evidence_observation_provenance_class(
    record: &EvidenceObservationRecord,
) -> CoreResult<EvidenceProvenanceClass> {
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
    Ok(evidence_provenance_class(source_kind, assurance_level))
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

fn final_acceptance_blocker(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    context: &CloseTaskContext,
) -> Result<Option<CloseReadinessBlocker>, PlanError> {
    let task_ref = task_ref_for_close(request, project_state.state_version);
    let Some(close_basis) = context.current_close_basis.as_ref() else {
        return Ok(Some(close_blocker(
            CloseReadinessBlockerCategory::FinalAcceptance,
            "missing_final_acceptance",
            "Final acceptance is required before completing the Task.",
            vec![task_ref.clone()],
            vec![NextActionSummary {
                action_kind: NextActionKind::RequestUserJudgment,
                owner_method: Some(MethodName::RequestUserJudgment),
                label: "Request final acceptance from the user.".to_owned(),
                blocking_question: None,
                required_refs: vec![task_ref],
            }],
        )));
    };
    let requirement = final_acceptance_requirement(close_basis);
    let authorities = resolved_judgment_authorities_for_context(
        store,
        project_state,
        request,
        context,
        JudgmentKind::FinalAcceptance,
    )?;
    if authorities
        .iter()
        .any(|authority| current_final_acceptance(authority, &requirement))
    {
        return Ok(None);
    }

    let stale_refs = non_current_judgment_refs_for_plan(
        store,
        project_state,
        request,
        JudgmentKind::FinalAcceptance,
    )?;
    let (code, message, related_refs) = if stale_refs.is_empty() {
        (
            "missing_final_acceptance",
            "Final acceptance is required before completing the Task.",
            vec![task_ref.clone()],
        )
    } else {
        (
            "stale_final_acceptance",
            "The available final acceptance is stale or incompatible with the current close basis.",
            refs_with_context(vec![task_ref.clone()], stale_refs),
        )
    };
    Ok(Some(close_blocker(
        CloseReadinessBlockerCategory::FinalAcceptance,
        code,
        message,
        related_refs,
        vec![NextActionSummary {
            action_kind: NextActionKind::RequestUserJudgment,
            owner_method: Some(MethodName::RequestUserJudgment),
            label: "Request current final acceptance from the user.".to_owned(),
            blocking_question: None,
            required_refs: vec![task_ref],
        }],
    )))
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
    let mut coverage = current_residual_risk_acceptance_coverage(
        &request.envelope.project_id,
        &request.task_id,
        project_state.state_version,
        basis,
        &authorities,
    );
    let stale_refs = non_current_judgment_refs_for_plan(
        store,
        project_state,
        request,
        JudgmentKind::ResidualRiskAcceptance,
    )?;
    if !stale_refs.is_empty() {
        for item in coverage.iter_mut().filter(|item| !item.accepted) {
            item.missing_reason = Some("stale_acceptance".to_owned()).into();
        }
    }
    Ok(coverage)
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
