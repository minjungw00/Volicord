use super::acceptance;
use super::change_control;
use super::evidence;
use super::facts::{
    acquire_close_readiness_facts, acquire_projected_store_facts, CloseReadinessFacts,
};
use super::policy::{self, CloseReadinessEvaluations};
use super::summary::{CloseReadinessAssessment, CloseReadinessSummary};
use super::CloseReadinessError;
use volicord_store::core_pipeline::{CoreProjectStore, ProjectStateHeader};
use volicord_types::ids::TaskId;
use volicord_types::schema::ToolEnvelope;
use volicord_types::values::{CloseIntent, UtcTimestamp};

/// Semantic close-readiness request independent of a public method body.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CloseReadinessRequest {
    pub(crate) envelope: ToolEnvelope,
    pub(crate) task_id: TaskId,
    pub(crate) intent: CloseIntent,
    pub(crate) superseding_task_id: Option<TaskId>,
}

impl CloseReadinessRequest {
    pub(crate) fn check(mut envelope: ToolEnvelope, task_id: TaskId) -> Self {
        envelope.task_id = Some(task_id.clone()).into();
        Self {
            envelope,
            task_id,
            intent: CloseIntent::Check,
            superseding_task_id: None,
        }
    }

    pub(crate) fn terminal(
        envelope: ToolEnvelope,
        task_id: TaskId,
        intent: CloseIntent,
        superseding_task_id: Option<TaskId>,
    ) -> Self {
        Self {
            envelope,
            task_id,
            intent,
            superseding_task_id,
        }
    }
}

/// Evaluates a method projection after acquiring only Store facts not supplied
/// by that projection.
pub(crate) fn plan_projected_close_readiness(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    mut facts: CloseReadinessFacts,
) -> Result<CloseReadinessSummary, CloseReadinessError> {
    acquire_projected_store_facts(store, task_id, &mut facts)?;
    let now = facts.now.clone();
    plan_close_readiness_with_facts(
        store,
        project_state,
        CloseReadinessRequest::check(envelope.clone(), task_id.clone()),
        &now,
        facts,
    )
}

/// Acquires current facts once and returns the method-neutral readiness view.
pub(crate) fn plan_close_readiness(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: CloseReadinessRequest,
    now: &UtcTimestamp,
) -> Result<CloseReadinessSummary, CloseReadinessError> {
    let facts = acquire_close_readiness_facts(
        store,
        project_state,
        &request.envelope,
        &request.task_id,
        now,
    )?;
    plan_close_readiness_with_facts(store, project_state, request, now, facts)
}

fn plan_close_readiness_with_facts(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: CloseReadinessRequest,
    now: &UtcTimestamp,
    facts: CloseReadinessFacts,
) -> Result<CloseReadinessSummary, CloseReadinessError> {
    evaluate_close_readiness_with_facts(store, project_state, request, now, facts)
        .map(CloseReadinessSummary::from)
}

fn evaluate_close_readiness_with_facts(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: CloseReadinessRequest,
    now: &UtcTimestamp,
    mut facts: CloseReadinessFacts,
) -> Result<CloseReadinessAssessment, CloseReadinessError> {
    let control_update = policy::resolve_control(&mut facts)?;
    let risk_acceptance_coverage =
        acceptance::risk_acceptance_coverage(store, project_state, &request, &mut facts)?;
    let terminal_change_control =
        change_control::terminal_blockers(store, project_state, &request, &mut facts, now)?;
    let terminal_acceptance =
        acceptance::terminal_blockers(store, project_state, &request, &mut facts, now)?;
    let completion = matches!(request.intent, CloseIntent::Check | CloseIntent::Complete);
    let completion_scope = if completion {
        change_control::completion_scope_blockers(store, project_state, &request, &facts)?
    } else {
        Vec::new()
    };
    let completion_authority = if completion {
        acceptance::completion_authority_blockers(store, project_state, &request, &mut facts, now)?
    } else {
        Vec::new()
    };
    let completion_basis = if completion {
        change_control::completion_basis_blockers(project_state, &request, &facts)?
    } else {
        Vec::new()
    };
    let completion_evidence = if completion {
        evidence::completion_blockers(store, project_state, &request, &facts)?
    } else {
        Vec::new()
    };
    let completion_acceptance = if completion {
        acceptance::completion_acceptance_blockers(
            store,
            project_state,
            &request,
            &mut facts,
            &risk_acceptance_coverage,
            !completion_evidence.is_empty(),
        )?
    } else {
        Vec::new()
    };
    let unrecorded_changes =
        change_control::unrecorded_change_blockers(project_state, &request, &facts);
    policy::combine(
        request.intent,
        project_state.state_version,
        facts,
        control_update,
        CloseReadinessEvaluations {
            risk_acceptance_coverage,
            terminal_change_control,
            terminal_acceptance,
            completion_scope,
            completion_authority,
            completion_basis,
            completion_evidence,
            completion_acceptance,
            unrecorded_changes,
        },
    )
}

/// Acquires current facts once and returns the close-operation assessment.
pub(crate) fn assess_close_readiness(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: CloseReadinessRequest,
    now: &UtcTimestamp,
) -> Result<CloseReadinessAssessment, CloseReadinessError> {
    let facts = acquire_close_readiness_facts(
        store,
        project_state,
        &request.envelope,
        &request.task_id,
        now,
    )?;
    evaluate_close_readiness_with_facts(store, project_state, request, now, facts)
}
