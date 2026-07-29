use crate::close_readiness::{close_next_action, plan_close_readiness, CloseReadinessRequest};
use crate::continuity::project_continuity_summary_from_record;
use crate::error_boundary::{
    store::{core_error_response, plan_error_response},
    user_action::user_action_service_plan_error,
};
use crate::method_execution::{prepare_or_response, PlanError};
use crate::method_rejection::validation_plan_error;
use crate::pipeline::{
    read_only_branch, CorePipelineError, CoreResult, CoreService, FreshnessPolicy,
    InvocationContext, MethodEffectPolicy, MethodPolicy, PipelineResponse, ReplayPolicy,
    TaskRequirement, VerifiedInvocationContext,
};
use crate::policy::close_readiness::is_terminal_lifecycle;
use crate::policy::workflow::{project_workflow_policy, resolve_task_control_authority};
use crate::projection::{
    active_acceptance_criteria_for_task, build_state_summary, changes_summary_text,
    close_state_summary_text, evidence_gate_summary_text, evidence_summary_for_display,
    guarantee_display_from_profile, next_actions_for_state, normalize_next_action_collection,
    primary_next_action, profile_summary_text, projected_blocker_refs, projected_evidence_summary,
    summary_card_for_core, unique_next_actions, write_ticket_summary_text, SummaryBuild,
    SummaryCardBuild,
};
use crate::record_refs::state_ref;
use crate::write_ticket::projected_write_ticket_summary;
use chrono::{DateTime, Utc};
use std::collections::BTreeSet;
use volicord_store::core_pipeline::{CoreProjectStore, ProjectStateHeader, TaskRecord};
use volicord_types::ids::{ProjectContinuityRecordId, ProjectId, TaskId};
use volicord_types::methods::{
    MethodOperationCategory, StatusInclude, StatusRequest, StatusResultFields, StatusStateSummary,
};
use volicord_types::schema::{
    AuthorityReceipt, CloseReadinessBlocker, ContinuityCursor, ContinuityPageInfo,
    ContinuityPageRequest, NextActionSummary, ProjectContinuityPage, RequiredNullable,
    TaskFlowItem, ToolEnvelope, DEFAULT_CONTINUITY_PAGE_SIZE, MAX_CONTINUITY_PAGE_SIZE,
};
use volicord_types::values::{
    AuthorityNextActor, CloseState, MethodName, OperationCategory, StateRecordKind,
    StatusCloseState, TaskLifecyclePhase, UtcTimestamp,
};
use volicord_user_action_service::{
    agent_safe_pending_user_action_summaries, projected_pending_user_action_refs,
};

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
            None,
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

        let continuity_page = match validated_continuity_page_request(
            &request,
            prepared.context.project_state.state_version,
        ) {
            Ok(page) => page,
            Err(error) => {
                return plan_error_response(
                    &request.envelope,
                    &prepared.context.project_state,
                    error,
                )
            }
        };

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
            StatusProjectionOptions {
                include: &request.include,
                continuity_page: continuity_page.as_ref(),
            },
            *prepared.operation_now.as_datetime(),
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
            .execute_prepared_request(prepared, read_only_branch::<StatusRequest>(result_fields))
        {
            Ok(response) => Ok(response),
            Err(error) => core_error_response(&request.envelope, Some(state_version), error),
        }
    }
}

struct StatusProjectionOptions<'a> {
    include: &'a StatusInclude,
    continuity_page: Option<&'a ContinuityPageRequest>,
}

fn validated_continuity_page_request(
    request: &StatusRequest,
    state_version: u64,
) -> Result<Option<ContinuityPageRequest>, PlanError> {
    let explicit_page = request
        .continuity_page
        .as_ref()
        .and_then(|page| page.as_ref());
    if !request.include.continuity {
        if explicit_page.is_some() {
            validation_plan_error(
                request.envelope.dry_run,
                Some(state_version),
                "continuity_page",
                "continuity_page must be null or omitted when continuity is not selected",
            )?;
        }
        return Ok(None);
    }

    let page = explicit_page.cloned().unwrap_or(ContinuityPageRequest {
        page_size: DEFAULT_CONTINUITY_PAGE_SIZE,
        cursor: RequiredNullable::null(),
    });
    if !(1..=MAX_CONTINUITY_PAGE_SIZE).contains(&page.page_size) {
        validation_plan_error(
            request.envelope.dry_run,
            Some(state_version),
            "continuity_page.page_size",
            "continuity_page.page_size must be between 1 and 64",
        )?;
    }
    if let Some(cursor) = page.cursor.as_ref() {
        if cursor.continuity_record_id.as_str().trim().is_empty() {
            validation_plan_error(
                request.envelope.dry_run,
                Some(state_version),
                "continuity_page.cursor.continuity_record_id",
                "continuity cursor record id must not be empty",
            )?;
        }
        if cursor
            .updated_at
            .ensure_canonical_rfc3339_representable()
            .is_err()
        {
            validation_plan_error(
                request.envelope.dry_run,
                Some(state_version),
                "continuity_page.cursor.updated_at",
                "continuity cursor timestamp must be representable as canonical RFC 3339 UTC",
            )?;
        }
    }
    Ok(Some(page))
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
    projection: StatusProjectionOptions<'_>,
    now: DateTime<Utc>,
) -> Result<StatusResultFields, PlanError> {
    let include = projection.include;
    let continuity_page_request = projection.continuity_page;
    let user_action_now = UtcTimestamp::from_datetime(now);
    let state_version = project_state.state_version;
    let project_id = &envelope.project_id;
    let mut active_task = None;
    let mut pending_user_action_summaries = Vec::new();
    let mut blocker_refs = Vec::new();
    let mut write_ticket_summary = None;
    let mut evidence_summary = None;
    let mut evidence_gate = None;
    let mut close_state = None;
    let mut current_close_basis = None;
    let mut risk_acceptance_coverage = None;
    let mut close_blockers = None;
    let mut continuity_summary = None;
    let mut task_flow = None;
    let mut authority_receipt = None;
    let mut next_actions = Vec::new();
    let mut receipt_next_actions = Vec::new();
    let mut card_pending_user_action_count = 0usize;
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
        let workflow_policy = project_workflow_policy(store).map_err(CorePipelineError::from)?;
        let resolved_control = resolve_task_control_authority(task, &workflow_policy)
            .map_err(CorePipelineError::from)?;
        let mut resolved_task = task.clone();
        if resolved_control.control_raised || resolved_control.acceptance_raised {
            resolved_task.effective_control_level = resolved_control.effective_control_level;
            resolved_task.control_level_reason = resolved_control.control_level_reason;
            resolved_task.acceptance_policy = resolved_control.acceptance_policy;
            resolved_task.acceptance_policy_reason = resolved_control.acceptance_policy_reason;
        }
        let task = &resolved_task;
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
                task.mode,
                &task_ref,
                change_unit_ref.as_ref(),
                state_version,
            )
        };
        let all_pending_user_actions =
            projected_pending_user_action_refs(store, &task_id, state_version, &user_action_now)
                .map_err(|error| user_action_service_plan_error(envelope, project_state, error))?;
        card_pending_user_action_count = all_pending_user_actions.len();
        if include.pending_user_actions {
            pending_user_action_summaries =
                agent_safe_pending_user_action_summaries(all_pending_user_actions.clone());
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
        let close_plan = plan_close_readiness(
            store,
            project_state,
            CloseReadinessRequest::check(envelope.project_id.clone(), task_id.clone()),
            &user_action_now,
        )
        .map_err(|error| {
            crate::error_boundary::close_readiness::close_readiness_plan_error(
                envelope,
                project_state,
                error,
            )
        })?;
        let lifecycle_phase = task.lifecycle_phase;
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
        if include.task {
            let state = build_state_summary(SummaryBuild {
                store,
                project_id,
                state_version,
                task,
                current_change_unit: current_change_unit.as_ref(),
                acceptance_criteria: active_acceptance_criteria_for_task(store, &task_id)?,
                pending_user_action_refs: all_pending_user_actions,
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
                guarantee_display: guarantee_projection.clone(),
            })?;
            active_task = Some(StatusStateSummary::from_state_summary(state, include));
        }
        let latest_run = store
            .run_observed_changes_for_task(&task_id)
            .map_err(CorePipelineError::from)?
            .into_iter()
            .find(|record| record.status == volicord_store::core_pipeline::RunStatus::Recorded);
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
        let completion_claim_allowed = current_close_basis.is_some()
            && effective_close_blockers.is_empty()
            && matches!(
                effective_close_state,
                CloseState::Ready | CloseState::Closed
            );
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
            completion_claim_allowed,
            next_actor: AuthorityNextActor::None,
            next_action: None,
        });
    }
    if include.continuity {
        let continuity_page_request = continuity_page_request.ok_or_else(|| {
            PlanError::Core(CorePipelineError::InvalidDispatch {
                detail: "selected continuity projection is missing its validated page request"
                    .to_owned(),
            })
        })?;
        continuity_summary = Some(projected_continuity_summary(
            store,
            state_version,
            continuity_page_request,
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
        profile: profile_summary_text(guarantee_projection.as_ref()),
        write_ticket: write_ticket_summary_text(
            include.write_ticket,
            write_ticket_summary.as_ref(),
        ),
        evidence: evidence_gate_summary_text(
            include.evidence || include.close,
            evidence_gate.as_ref(),
        ),
        pending_user_actions: card_pending_user_action_count,
        changes: changes_summary_text(
            include.close,
            if include.close {
                volicord_store::guards::list_unresolved_unrecorded_changes(
                    store.runtime_home(),
                    project_id.as_str(),
                    None,
                )
                .map_err(CorePipelineError::from)?
                .len() as u64
            } else {
                0
            },
        ),
        close_status: close_state_summary_text(include.close, close_state),
        verified_invocation,
        next_action: primary_next_action(&next_actions, close_blockers_slice),
    });

    Ok(StatusResultFields {
        summary_card,
        active_task,
        status_summary: status_summary_for(task, close_state, close_blockers.as_deref()),
        next_actions,
        pending_user_action_summaries,
        blocker_refs,
        write_ticket_summary,
        evidence_summary: include.evidence.then(|| evidence_summary.into()),
        evidence_gate: (include.evidence || include.close).then(|| evidence_gate.into()),
        close_state,
        current_close_basis: include.close.then(|| current_close_basis.into()),
        risk_acceptance_coverage,
        close_blockers,
        guarantee_display: guarantee_projection.map(RequiredNullable::some),
        continuity_summary,
        task_flow,
        authority_receipt,
    })
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
                relation: record.lineage_relation,
                mode: record.mode,
                work_phase: record.work_phase,
                lifecycle_phase: record.lifecycle_phase,
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
    request: &ContinuityPageRequest,
) -> Result<ProjectContinuityPage, PlanError> {
    let stored_page = store
        .active_project_continuity_page(request.page_size, request.cursor.as_ref())
        .map_err(CorePipelineError::from)?;
    let items = stored_page
        .records
        .iter()
        .map(|record| {
            project_continuity_summary_from_record(record, state_version).map_err(PlanError::Core)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let returned_count = u64::try_from(items.len()).map_err(|_| {
        PlanError::Core(CorePipelineError::InvalidDispatch {
            detail: "continuity page item count cannot be represented in the public response"
                .to_owned(),
        })
    })?;
    let next_cursor = if stored_page.truncated {
        let last = stored_page.records.last().ok_or_else(|| {
            PlanError::Core(CorePipelineError::InvalidDispatch {
                detail: "truncated continuity page has no cursor source record".to_owned(),
            })
        })?;
        RequiredNullable::some(ContinuityCursor {
            updated_at: last.updated_at.clone(),
            continuity_record_id: ProjectContinuityRecordId::new(last.continuity_record_id.clone()),
        })
    } else {
        RequiredNullable::null()
    };
    Ok(ProjectContinuityPage {
        items,
        page_info: ContinuityPageInfo {
            total_count: stored_page.total_count,
            returned_count,
            truncated: stored_page.truncated,
            next_cursor,
        },
    })
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
