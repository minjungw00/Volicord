use volicord_types::ids::{AcceptanceCriterionId, BaselineRef, ChangeUnitId, ProjectId, TaskId};
use volicord_types::methods::UpdateScopeRequest;
use volicord_types::schema::{
    AcceptanceCriterion, CloseReadinessBlocker, CurrentCloseBasis, EvidenceGateSummary,
    EvidenceSummary, GuaranteeDisplay, JsonObject, NextActionSummary, ProjectEnforcementProfile,
    RequiredNullable, StateRecordRef, SummaryCard, TaskLifecycleState, TaskLineageSummary,
    WorkspaceContext, WriteTicketStateSummary,
};
use volicord_types::values::{
    ActorSource, CloseReason, CloseState, EvidenceDisplayState, EvidenceGateState, GuaranteeLevel,
    MethodName, NextActionKind, NextActionPresentationRole, OperationCategory, StateRecordKind,
    StatusCloseState, TaskLifecyclePhase, TaskMode, TaskResult, WorkspaceVcs, WriteTicketStatus,
};

use serde_json::Value;
use std::collections::BTreeSet;
use volicord_store::core_pipeline::{
    AcceptanceCriterionRecord, ChangeUnitInsert, ChangeUnitRecord, ChangeUnitStatus,
    CoreProjectStore, CoreStorageMutation, ProjectStateHeader, StoredChangeUnitLifecycle,
    StoredChangeUnitScopeSummary, StoredChangeUnitWriteBasis, TaskMutation, TaskRecord,
    TaskScopeUpdate,
};

use crate::evidence_facts;
use crate::pipeline::{CorePipelineError, CoreResult, VerifiedInvocationContext};
use crate::policy::evidence::{state_record_ref_identity_key, unique_state_record_refs};
use crate::policy::{
    close_readiness_evidence::{project_close_evidence_summary, required_acceptance_criterion_ids},
    workflow::project_workflow_policy,
};
use crate::record_refs::{state_ref, stored_refs_to_state_refs};
use crate::task_state::StoredScope;
use crate::write_ticket::change_unit_effect_contract;

pub(crate) fn acceptance_criterion_from_record(
    record: &AcceptanceCriterionRecord,
) -> CoreResult<AcceptanceCriterion> {
    Ok(AcceptanceCriterion {
        acceptance_criterion_id: AcceptanceCriterionId::new(record.acceptance_criterion_id.clone()),
        statement: record.statement.clone(),
        evidence_requirement: record.evidence_requirement,
    })
}

pub(crate) fn active_acceptance_criteria_for_task(
    store: &CoreProjectStore,
    task_id: &TaskId,
) -> CoreResult<Vec<AcceptanceCriterion>> {
    store
        .active_acceptance_criteria(task_id)
        .map_err(CorePipelineError::from)?
        .iter()
        .map(acceptance_criterion_from_record)
        .collect()
}

pub(crate) struct SummaryBuild<'a> {
    pub(crate) store: &'a CoreProjectStore<'a>,
    pub(crate) project_id: &'a ProjectId,
    pub(crate) state_version: u64,
    pub(crate) task: &'a TaskRecord,
    pub(crate) current_change_unit: Option<&'a ChangeUnitRecord>,
    pub(crate) acceptance_criteria: Vec<AcceptanceCriterion>,
    pub(crate) pending_user_action_refs: Vec<StateRecordRef>,
    pub(crate) blocker_refs: Vec<StateRecordRef>,
    pub(crate) write_ticket_summary: Option<WriteTicketStateSummary>,
    pub(crate) evidence_summary: Option<EvidenceSummary>,
    pub(crate) evidence_gate: Option<EvidenceGateSummary>,
    pub(crate) close_state: Option<CloseState>,
    pub(crate) close_blockers: Vec<CloseReadinessBlocker>,
    pub(crate) guarantee_display: Option<GuaranteeDisplay>,
}

pub(crate) fn build_state_summary(
    input: SummaryBuild<'_>,
) -> CoreResult<volicord_types::schema::StateSummary> {
    let SummaryBuild {
        store,
        project_id,
        state_version,
        task,
        current_change_unit,
        acceptance_criteria,
        pending_user_action_refs,
        blocker_refs,
        write_ticket_summary,
        evidence_summary,
        evidence_gate,
        close_state,
        close_blockers,
        guarantee_display,
    } = input;
    let workflow_policy = project_workflow_policy(store).map_err(CorePipelineError::from)?;
    let task_id = TaskId::new(task.task_id.clone());
    let task_ref = state_ref(
        StateRecordKind::Task,
        &task.task_id,
        project_id,
        Some(&task_id),
        Some(state_version),
    );
    let active_change_unit_ref = current_change_unit.map(|record| {
        state_ref(
            StateRecordKind::ChangeUnit,
            &record.change_unit_id,
            project_id,
            Some(&task_id),
            Some(record.basis_state_version),
        )
    });
    let effect_contract = current_change_unit
        .map(change_unit_effect_contract)
        .transpose()?
        .flatten();
    let workspace_context = current_change_unit
        .and_then(|record| record.write_basis.git_workspace_context.as_ref())
        .map(|workspace| WorkspaceContext {
            vcs: WorkspaceVcs::Git,
            git_common_dir: workspace.git_common_dir.clone(),
            worktree_id: workspace.worktree_id.clone(),
            branch_ref: workspace.branch_ref.clone(),
            head_sha: workspace.head_sha.clone(),
            workspace_fingerprint: workspace.workspace_fingerprint.clone(),
        });
    let lineage = match (
        task.predecessor_task_id.as_ref(),
        task.lineage_relation,
        task.lineage_reason.as_ref(),
    ) {
        (Some(predecessor_task_id), Some(relation), Some(creation_reason)) => {
            Some(TaskLineageSummary {
                predecessor_task_ref: state_ref(
                    StateRecordKind::Task,
                    predecessor_task_id,
                    project_id,
                    Some(&TaskId::new(predecessor_task_id.clone())),
                    Some(state_version),
                ),
                relation,
                creation_reason: creation_reason.clone(),
                carry_forward: task.carry_forward.clone(),
            })
        }
        (None, None, None) => None,
        _ => return invalid_storage("tasks.lineage"),
    };
    let scope = StoredScope::from_task(task)?;
    let change_unit_scope =
        current_change_unit.and_then(|record| record.scope_summary.scope_summary.clone());
    Ok(volicord_types::schema::StateSummary {
        project_id: project_id.clone(),
        state_version,
        task_ref: Some(task_ref),
        mode: Some(task.mode),
        requested_control_level: Some(task.requested_control_level),
        effective_control_level: Some(task.effective_control_level),
        control_level_reason: Some(task.control_level_reason.clone()),
        project_policy: workflow_policy.summary,
        work_phase: Some(task.work_phase),
        acceptance_policy: Some(task.acceptance_policy),
        acceptance_policy_reason: Some(task.acceptance_policy_reason.clone()),
        lineage,
        lifecycle: Some(TaskLifecycleState {
            lifecycle_phase: task.lifecycle_phase,
            close_reason: parse_close_reason(task)?,
            result: task.result.unwrap_or(TaskResult::None),
            closed_at: task.closed_at.clone(),
        }),
        scope_revision: task.scope_revision,
        goal_summary: scope.goal_summary,
        scope_summary: change_unit_scope.or(scope.scope_summary),
        non_goals: scope.non_goals,
        acceptance_criteria,
        autonomy_boundary: scope.autonomy_boundary,
        active_change_unit_ref,
        effect_contract,
        baseline_ref: scope.baseline_ref.map(BaselineRef::new),
        workspace_context,
        shaping_readiness: None,
        pending_user_action_summaries:
            volicord_user_action_service::agent_safe_pending_user_action_summaries(
                pending_user_action_refs,
            ),
        blocker_refs,
        write_ticket_summary,
        evidence_summary,
        evidence_gate,
        close_state,
        close_blockers,
        guarantee_display,
    })
}

pub(crate) fn guarantee_display_for_invocation(
    store: &CoreProjectStore,
    verified_invocation: &VerifiedInvocationContext,
    state_version: u64,
) -> CoreResult<GuaranteeDisplay> {
    let profile = store
        .project_enforcement_profile()
        .map_err(CorePipelineError::from)?
        .profile;
    Ok(guarantee_display_from_profile(
        &profile,
        verified_invocation,
        state_version,
    ))
}

pub(crate) fn guarantee_display_from_profile(
    profile: &ProjectEnforcementProfile,
    verified_invocation: &VerifiedInvocationContext,
    state_version: u64,
) -> GuaranteeDisplay {
    GuaranteeDisplay {
        level: profile.guarantee_level,
        basis: format!(
            "Project enforcement profile `{}` is active for actor source `{}` operation category `{}` verified by `{}`; enabled mechanisms: none; no stronger enforcement is active.",
            profile.profile_id,
            verified_invocation.actor_source,
            verified_invocation.operation_category.as_str(),
            verified_invocation.verification_basis
        ),
        capability_refs: vec![invocation_binding_ref(verified_invocation, state_version)],
    }
}

pub(crate) fn invocation_binding_ref(
    verified_invocation: &VerifiedInvocationContext,
    state_version: u64,
) -> StateRecordRef {
    match &verified_invocation.actor_source {
        ActorSource::AgentConnection(connection_id) => state_ref(
            StateRecordKind::AgentConnection,
            connection_id.as_str(),
            &verified_invocation.project_id,
            None,
            Some(state_version),
        ),
        ActorSource::LocalUser | ActorSource::System => state_ref(
            StateRecordKind::ProjectState,
            verified_invocation
                .actor_source
                .to_canonical_string()
                .as_str(),
            &verified_invocation.project_id,
            None,
            Some(state_version),
        ),
    }
}

pub(crate) fn projected_evidence_summary(
    store: &CoreProjectStore,
    project_id: &ProjectId,
    state_version: u64,
    task: &TaskRecord,
) -> CoreResult<Option<EvidenceSummary>> {
    let task_id = TaskId::new(task.task_id.clone());
    let record = store
        .latest_evidence_summary(&task_id)
        .map_err(CorePipelineError::from)?;
    let required = evidence_facts::load_required_evidence_criterion_ids(store, &task_id)?;
    let facts = evidence_facts::load_close_evidence_summary_facts(
        store,
        record.as_ref(),
        task,
        project_id,
        &task_id,
        state_version,
    )?;
    Ok(project_close_evidence_summary(facts, &required))
}

pub(crate) fn projected_evidence_summary_for_criteria(
    store: &CoreProjectStore,
    project_id: &ProjectId,
    state_version: u64,
    task: &TaskRecord,
    acceptance_criteria: &[AcceptanceCriterion],
) -> CoreResult<Option<EvidenceSummary>> {
    let task_id = TaskId::new(task.task_id.clone());
    let record = store
        .latest_evidence_summary(&task_id)
        .map_err(CorePipelineError::from)?;
    let required = required_acceptance_criterion_ids(acceptance_criteria);
    let facts = evidence_facts::load_close_evidence_summary_facts(
        store,
        record.as_ref(),
        task,
        project_id,
        &task_id,
        state_version,
    )?;
    Ok(project_close_evidence_summary(facts, &required))
}

pub(crate) fn projected_blocker_refs(
    store: &CoreProjectStore,
    task_id: &TaskId,
    state_version: u64,
) -> CoreResult<Vec<StateRecordRef>> {
    Ok(stored_refs_to_state_refs(
        store
            .active_blocker_refs(task_id, state_version)
            .map_err(CorePipelineError::from)?,
    ))
}

pub(crate) fn projected_close_basis(
    store: &CoreProjectStore,
    task_id: &TaskId,
) -> CoreResult<Option<CurrentCloseBasis>> {
    Ok(store
        .task_revision_record(task_id)
        .map_err(CorePipelineError::from)?
        .and_then(|record| record.current_close_basis))
}

pub(crate) fn project_state_projection(
    project_state: &ProjectStateHeader,
    state_version: u64,
    active_task_id: Option<String>,
) -> ProjectStateHeader {
    ProjectStateHeader {
        project_id: project_state.project_id.clone(),
        state_version,
        active_task_id,
        updated_at: project_state.updated_at.clone(),
    }
}

pub(crate) fn change_unit_insert(
    request: &UpdateScopeRequest,
    change_unit_id: &ChangeUnitId,
    verified_invocation: &VerifiedInvocationContext,
) -> CoreResult<ChangeUnitInsert> {
    let fields = &request.change_unit.fields;
    let scope_summary = string_member(fields, "scope_summary")
        .or_else(|| request.scope_boundary.as_ref().cloned())
        .unwrap_or_else(|| "Current Change Unit".to_owned());
    let affected_areas = string_array_member(fields, "affected_areas");
    let affected_paths = string_array_member(fields, "affected_paths");
    let constraints = string_array_member(fields, "constraints");
    Ok(ChangeUnitInsert {
        change_unit_id: change_unit_id.as_str().to_owned(),
        task_id: request.task_id.as_str().to_owned(),
        scope_summary: StoredChangeUnitScopeSummary {
            scope_summary: Some(scope_summary),
            affected_areas,
            constraints,
        },
        bounded_paths: affected_paths,
        write_basis: StoredChangeUnitWriteBasis {
            baseline_ref: request.baseline_ref.clone().into_option(),
            git_workspace_context: verified_invocation.git_workspace_context.as_ref().map(
                |context| volicord_store::core_pipeline::StoredGitWorkspaceContext {
                    git_common_dir: context.git_common_dir.clone(),
                    worktree_id: context.worktree_id.clone(),
                    branch_ref: context.branch_ref.clone(),
                    head_sha: context.head_sha.clone(),
                    workspace_fingerprint: context.workspace_fingerprint.clone(),
                },
            ),
        },
        effect_contract: request.change_unit.effect_contract.clone(),
        lifecycle: StoredChangeUnitLifecycle {
            recovery_required: false,
        },
    })
}

pub(crate) fn synthetic_change_unit_record(
    project_id: &ProjectId,
    task_id: &TaskId,
    insert: &ChangeUnitInsert,
    planned_state_version: u64,
) -> CoreResult<ChangeUnitRecord> {
    Ok(ChangeUnitRecord {
        project_id: project_id.as_str().to_owned(),
        change_unit_id: insert.change_unit_id.clone(),
        task_id: task_id.as_str().to_owned(),
        status: ChangeUnitStatus::Active,
        is_current: true,
        basis_state_version: planned_state_version,
        scope_summary: insert.scope_summary.clone(),
        bounded_paths: insert.bounded_paths.clone(),
        write_basis: insert.write_basis.clone(),
        effect_contract: insert.effect_contract.clone(),
        lifecycle: insert.lifecycle.clone(),
    })
}

pub(crate) fn next_actions_for_state(
    task_mode: TaskMode,
    task_ref: &StateRecordRef,
    change_unit_ref: Option<&StateRecordRef>,
    expected_state_version: u64,
) -> Vec<NextActionSummary> {
    match (task_mode, change_unit_ref) {
        (TaskMode::Advisor, Some(change_unit_ref)) => vec![NextActionSummary {
            presentation_role: NextActionPresentationRole::Primary,
            action_kind: NextActionKind::RecordRun,
            owner_method: Some(MethodName::RecordRun),
            allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
            label: "Record an advisory shaping update for the current Change Unit.".to_owned(),
            blocking_question: None,
            expected_state_version: RequiredNullable::some(expected_state_version),
            required_refs: vec![task_ref.clone(), change_unit_ref.clone()],
        }],
        (_, Some(change_unit_ref)) => vec![NextActionSummary {
            presentation_role: NextActionPresentationRole::Primary,
            action_kind: NextActionKind::PrepareWrite,
            owner_method: Some(MethodName::PrepareWrite),
            allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
            label: "Check the current change against current scope.".to_owned(),
            blocking_question: None,
            expected_state_version: RequiredNullable::some(expected_state_version),
            required_refs: vec![task_ref.clone(), change_unit_ref.clone()],
        }],
        (TaskMode::Advisor, None) => vec![NextActionSummary {
            presentation_role: NextActionPresentationRole::Primary,
            action_kind: NextActionKind::UpdateScope,
            owner_method: Some(MethodName::UpdateScope),
            allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
            label:
                "Create the first currently applied Change Unit before recording advisory shaping."
                    .to_owned(),
            blocking_question: None,
            expected_state_version: RequiredNullable::some(expected_state_version),
            required_refs: vec![task_ref.clone()],
        }],
        (_, None) => vec![NextActionSummary {
            presentation_role: NextActionPresentationRole::Primary,
            action_kind: NextActionKind::UpdateScope,
            owner_method: Some(MethodName::UpdateScope),
            allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
            label:
                "Create the first currently applied Change Unit before write-ticket preparation."
                    .to_owned(),
            blocking_question: None,
            expected_state_version: RequiredNullable::some(expected_state_version),
            required_refs: vec![task_ref.clone()],
        }],
    }
}

pub(crate) fn task_lifecycle_mutation(
    task_id: &TaskId,
    lifecycle_phase: TaskLifecyclePhase,
) -> CoreStorageMutation {
    CoreStorageMutation::Task(TaskMutation::UpdateScope(TaskScopeUpdate {
        task_id: task_id.as_str().to_owned(),
        work_phase: None,
        lifecycle_phase: Some(lifecycle_phase),
        result: None,
        title: None,
        summary: None,
        shaping: None,
        bounded_context: None,
        autonomy_boundary: None,
        close_summary: None,
    }))
}

pub(crate) fn summary_card_for_core(input: SummaryCardBuild<'_>) -> SummaryCard {
    let next = input
        .next_action
        .map(next_action_label)
        .unwrap_or_else(|| "none".to_owned());
    SummaryCard {
        task: task_summary_text(input.task),
        recording: input.recording.to_owned(),
        profile: input.profile.unwrap_or_else(|| "not_selected".to_owned()),
        write_ticket: input.write_ticket,
        evidence: input.evidence,
        user_action: count_state_text("pending", input.pending_user_actions),
        changes: input.changes,
        close_status: input.close_status,
        transport: transport_summary(input.verified_invocation),
        next,
        next_action: input.next_action.cloned(),
        guarantee: AUTHORITY_RECORD_SUMMARY_GUARANTEE.to_owned(),
    }
}

pub(crate) struct SummaryCardBuild<'a> {
    pub(crate) task: Option<&'a TaskRecord>,
    pub(crate) recording: &'a str,
    pub(crate) profile: Option<String>,
    pub(crate) write_ticket: String,
    pub(crate) evidence: String,
    pub(crate) pending_user_actions: usize,
    pub(crate) changes: String,
    pub(crate) close_status: String,
    pub(crate) verified_invocation: &'a VerifiedInvocationContext,
    pub(crate) next_action: Option<&'a NextActionSummary>,
}

const AUTHORITY_RECORD_SUMMARY_GUARANTEE: &str =
    "Local authority record; not OS enforcement, correctness proof, test sufficiency proof, or review completion.";

pub(crate) fn task_summary_text(task: Option<&TaskRecord>) -> String {
    task.map(|task| {
        format!(
            "selected ({})",
            task_lifecycle_phase_storage(task.lifecycle_phase)
        )
    })
    .unwrap_or_else(|| "none".to_owned())
}

pub(crate) fn profile_summary_text(guarantee_display: Option<&GuaranteeDisplay>) -> Option<String> {
    guarantee_display.map(|display| match display.level {
        GuaranteeLevel::Cooperative => "record".to_owned(),
    })
}

pub(crate) fn write_ticket_summary_text(
    selected: bool,
    summary: Option<&WriteTicketStateSummary>,
) -> String {
    if !selected {
        return "not_selected".to_owned();
    }
    summary
        .map(|summary| match summary.status {
            WriteTicketStatus::Active => "active",
            WriteTicketStatus::Consumed => "consumed",
            WriteTicketStatus::Invalidated => "invalidated",
            WriteTicketStatus::Revoked => "revoked",
        })
        .unwrap_or("none")
        .to_owned()
}

pub(crate) fn evidence_summary_for_display(
    mut summary: EvidenceSummary,
    close_basis: Option<&CurrentCloseBasis>,
) -> EvidenceSummary {
    summary.evidence_state = if close_basis
        .and_then(|basis| basis.evidence_summary_ref.as_ref())
        .is_some()
    {
        Some(EvidenceDisplayState::AcceptedForClose)
    } else if evidence_summary_has_attached_evidence(&summary) {
        Some(EvidenceDisplayState::Attached)
    } else {
        None
    };
    summary
}

pub(crate) fn evidence_summary_has_attached_evidence(summary: &EvidenceSummary) -> bool {
    summary.updated_by_run_ref.is_some()
        || !summary.artifact_refs.is_empty()
        || !summary.observation_refs.is_empty()
        || summary.coverage_items.iter().any(|item| {
            !item.supporting_run_refs.is_empty()
                || !item.observation_refs.is_empty()
                || !item.supporting_artifact_refs.is_empty()
        })
}

pub(crate) fn evidence_gate_summary_text(
    selected: bool,
    summary: Option<&EvidenceGateSummary>,
) -> String {
    if !selected {
        return "not_selected".to_owned();
    }
    summary
        .map(|summary| evidence_gate_state_text(summary.state))
        .unwrap_or("none")
        .to_owned()
}

pub(crate) fn evidence_gate_state_text(state: EvidenceGateState) -> &'static str {
    match state {
        EvidenceGateState::NotRequired => "not_required",
        EvidenceGateState::OptionalNone => "optional_none",
        EvidenceGateState::RequiredMissing => "required_missing",
        EvidenceGateState::Partial => "partial",
        EvidenceGateState::Sufficient => "sufficient",
        EvidenceGateState::Stale => "stale",
        EvidenceGateState::Blocked => "blocked",
    }
}

pub(crate) fn close_state_summary_text(
    selected: bool,
    close_state: Option<StatusCloseState>,
) -> String {
    if !selected {
        return "not_selected".to_owned();
    }
    close_state
        .map(status_close_state_text)
        .unwrap_or("none")
        .to_owned()
}

pub(crate) fn status_close_state_text(close_state: StatusCloseState) -> &'static str {
    match close_state {
        StatusCloseState::Ready => "ready",
        StatusCloseState::Blocked => "blocked",
        StatusCloseState::Closed => "closed",
        StatusCloseState::Cancelled => "cancelled",
        StatusCloseState::Superseded => "superseded",
        StatusCloseState::None => "none",
    }
}

pub(crate) fn close_state_text(close_state: CloseState) -> &'static str {
    match close_state {
        CloseState::Ready => "ready",
        CloseState::Blocked => "blocked",
        CloseState::Closed => "closed",
        CloseState::Cancelled => "cancelled",
        CloseState::Superseded => "superseded",
    }
}

pub(crate) fn changes_summary_text(selected: bool, unresolved_count: u64) -> String {
    if !selected {
        return "not_selected".to_owned();
    }
    count_state_text("unresolved", unresolved_count as usize)
}

pub(crate) fn count_state_text(label: &str, count: usize) -> String {
    if count == 0 {
        "none".to_owned()
    } else {
        format!("{label} ({count})")
    }
}

pub(crate) fn next_action_label(action: &NextActionSummary) -> String {
    if !action.label.trim().is_empty() {
        action.label.clone()
    } else {
        action
            .blocking_question
            .clone()
            .unwrap_or_else(|| "none".to_owned())
    }
}

pub(crate) fn normalize_next_action_collection(
    actions: &mut [NextActionSummary],
    expected_state_version: u64,
) {
    for (index, action) in actions.iter_mut().enumerate() {
        action.presentation_role = if index == 0 {
            NextActionPresentationRole::Primary
        } else {
            NextActionPresentationRole::Additional
        };
        action.allowed_operation_categories = allowed_operation_categories(action.owner_method);
        action.expected_state_version = next_action_expected_state_version(
            &action.allowed_operation_categories,
            expected_state_version,
        );
    }
}

pub(crate) fn unique_next_actions(actions: Vec<NextActionSummary>) -> Vec<NextActionSummary> {
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
            .expect("serializing the closed action identity tuple cannot fail");
            seen.insert(key).then_some(action)
        })
        .collect()
}

pub(crate) fn next_action_expected_state_version(
    allowed_operation_categories: &[OperationCategory],
    expected_state_version: u64,
) -> RequiredNullable<u64> {
    if allowed_operation_categories.contains(&OperationCategory::AgentWorkflow) {
        RequiredNullable::some(expected_state_version)
    } else {
        RequiredNullable::null()
    }
}

pub(crate) fn allowed_operation_categories(
    owner_method: Option<MethodName>,
) -> Vec<OperationCategory> {
    match owner_method {
        Some(MethodName::ResolveUserAction) => {
            vec![OperationCategory::UserOnly]
        }
        Some(MethodName::ReconcileChanges) => vec![
            OperationCategory::AgentWorkflow,
            OperationCategory::LocalRecovery,
        ],
        Some(
            MethodName::UpdateScope
            | MethodName::PrepareEvidenceCapture
            | MethodName::PrepareWrite
            | MethodName::StageArtifact
            | MethodName::RecordRun
            | MethodName::RequestUserAction
            | MethodName::CloseTask,
        ) => vec![OperationCategory::AgentWorkflow],
        Some(
            MethodName::Intake
            | MethodName::Status
            | MethodName::GetOperationResult
            | MethodName::CheckClose,
        )
        | None => Vec::new(),
    }
}

pub(crate) fn primary_next_action<'a>(
    next_actions: &'a [NextActionSummary],
    close_blockers: &'a [CloseReadinessBlocker],
) -> Option<&'a NextActionSummary> {
    next_actions
        .iter()
        .find(|action| action.presentation_role == NextActionPresentationRole::Primary)
        .or_else(|| {
            close_blockers
                .iter()
                .flat_map(|blocker| blocker.next_actions.iter())
                .find(|action| action.presentation_role == NextActionPresentationRole::Primary)
        })
}

pub(crate) fn transport_summary(verified_invocation: &VerifiedInvocationContext) -> String {
    match &verified_invocation.actor_source {
        ActorSource::AgentConnection(_) => "Agent Connection".to_owned(),
        ActorSource::LocalUser => "User Channel".to_owned(),
        ActorSource::System => "system".to_owned(),
    }
}

pub(crate) fn parse_close_reason(task: &TaskRecord) -> CoreResult<CloseReason> {
    Ok(task.close_summary.close_reason)
}

pub(crate) fn task_lifecycle_phase_storage(value: TaskLifecyclePhase) -> &'static str {
    match value {
        TaskLifecyclePhase::Shaping => "shaping",
        TaskLifecyclePhase::Ready => "ready",
        TaskLifecyclePhase::Executing => "executing",
        TaskLifecyclePhase::WaitingUser => "waiting_user",
        TaskLifecyclePhase::Blocked => "blocked",
        TaskLifecyclePhase::Completed => "completed",
        TaskLifecyclePhase::Cancelled => "cancelled",
        TaskLifecyclePhase::Superseded => "superseded",
    }
}

pub(crate) fn invalid_storage<T>(field: &'static str) -> CoreResult<T> {
    Err(CorePipelineError::Invariant {
        detail: format!("typed Store facts violate the Core `{field}` invariant"),
    })
}

pub(crate) fn string_member(object: &JsonObject, key: &str) -> Option<String> {
    object.get(key).and_then(Value::as_str).map(str::to_owned)
}

pub(crate) fn string_array_member(object: &JsonObject, key: &str) -> Vec<String> {
    object
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use volicord_types::{
        ids::{ProjectId, RecordId, TaskId},
        schema::RequiredNullable,
        values::{NextActionKind, NextActionPresentationRole},
    };

    #[test]
    fn semantic_next_actions_are_normalized_deduplicated_and_selected_by_role() {
        for owner_method in [
            MethodName::UpdateScope,
            MethodName::PrepareWrite,
            MethodName::StageArtifact,
            MethodName::RecordRun,
            MethodName::RequestUserAction,
            MethodName::CloseTask,
        ] {
            assert_eq!(
                allowed_operation_categories(Some(owner_method)),
                vec![OperationCategory::AgentWorkflow]
            );
        }
        assert_eq!(
            allowed_operation_categories(Some(MethodName::ResolveUserAction)),
            vec![OperationCategory::UserOnly]
        );
        assert_eq!(
            allowed_operation_categories(Some(MethodName::ReconcileChanges)),
            vec![
                OperationCategory::AgentWorkflow,
                OperationCategory::LocalRecovery,
            ]
        );
        assert!(allowed_operation_categories(None).is_empty());

        let primary = NextActionSummary {
            presentation_role: NextActionPresentationRole::Primary,
            action_kind: NextActionKind::RecordRun,
            owner_method: Some(MethodName::RecordRun),
            allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
            label: "Record the current result.".to_owned(),
            blocking_question: None,
            expected_state_version: RequiredNullable::null(),
            required_refs: Vec::new(),
        };
        let mut additional_duplicate = primary.clone();
        additional_duplicate.presentation_role = NextActionPresentationRole::Additional;
        additional_duplicate.expected_state_version = RequiredNullable::some(41);

        let deduplicated = unique_next_actions(vec![additional_duplicate.clone(), primary.clone()]);
        assert_eq!(deduplicated.len(), 1);

        let distinct_additional = NextActionSummary {
            label: "Additional action.".to_owned(),
            ..additional_duplicate
        };
        let reordered = [distinct_additional, primary.clone()];
        let selected = primary_next_action(&reordered, &[])
            .expect("primary action should be selected by role");
        assert_eq!(selected, &primary);

        let older_ref = StateRecordRef {
            record_kind: StateRecordKind::Task,
            record_id: RecordId::new("task_same_identity"),
            project_id: ProjectId::new("project_projection"),
            task_id: Some(TaskId::new("task_context_old")).into(),
            produced_at_state_version: Some(3).into(),
        };
        let newer_ref = StateRecordRef {
            task_id: Some(TaskId::new("task_context_new")).into(),
            produced_at_state_version: Some(8).into(),
            ..older_ref.clone()
        };
        let deduplicated_refs = unique_next_actions(vec![NextActionSummary {
            required_refs: vec![newer_ref.clone(), older_ref],
            ..primary.clone()
        }]);
        assert_eq!(deduplicated_refs[0].required_refs, vec![newer_ref]);

        let mut user_only_action = NextActionSummary {
            owner_method: Some(MethodName::ResolveUserAction),
            expected_state_version: RequiredNullable::some(99),
            ..primary.clone()
        };
        normalize_next_action_collection(std::slice::from_mut(&mut user_only_action), 8);
        assert_eq!(
            user_only_action.allowed_operation_categories,
            vec![OperationCategory::UserOnly]
        );
        assert!(user_only_action.expected_state_version.is_none());

        let mut read_action = NextActionSummary {
            owner_method: Some(MethodName::Status),
            expected_state_version: RequiredNullable::some(99),
            ..primary
        };
        normalize_next_action_collection(std::slice::from_mut(&mut read_action), 8);
        assert!(read_action.allowed_operation_categories.is_empty());
        assert!(read_action.expected_state_version.is_none());
    }
}
