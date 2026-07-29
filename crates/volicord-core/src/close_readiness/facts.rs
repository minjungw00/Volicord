use super::CloseReadinessError;
use crate::acceptance_facts::active_acceptance_criteria;
use crate::evidence_facts::load_close_evidence_summary_facts;
use crate::pipeline::{CorePipelineError, CoreResult};
use crate::policy::close_readiness_evidence::{
    project_close_evidence_summary, required_acceptance_criterion_ids,
};
use crate::policy::workflow::{project_workflow_policy, ProjectWorkflowPolicy};
use crate::record_refs::state_ref_from_stored;
use std::collections::{BTreeSet, HashMap};
use volicord_store::core_pipeline::{
    ChangeUnitRecord, CoreProjectStore, ProjectStateHeader, StoredWriteTicket, TaskRecord,
};
use volicord_store::guards::UnrecordedChangeRecord;
use volicord_types::ids::{ProjectId, TaskId};
use volicord_types::schema::{
    AcceptanceCriterion, ArtifactRef, CurrentCloseBasis, EvidenceObservation, EvidenceSummary,
    StateRecordRef,
};
use volicord_types::values::{JudgmentKind, UtcTimestamp};
use volicord_user_action_service::UserActionAuthority;

/// Typed close-readiness facts acquired from Store state or a method projection.
pub(crate) struct CloseReadinessFacts {
    pub(crate) now: UtcTimestamp,
    pub(crate) task: TaskRecord,
    pub(crate) current_change_unit: Option<ChangeUnitRecord>,
    pub(crate) current_close_basis: Option<CurrentCloseBasis>,
    pub(crate) pending_user_action_refs: Vec<StateRecordRef>,
    pub(crate) blocker_refs: Vec<StateRecordRef>,
    pub(crate) evidence_summary: Option<EvidenceSummary>,
    pub(crate) acceptance_criteria: Option<Vec<AcceptanceCriterion>>,
    pub(crate) artifact_refs: Vec<ArtifactRef>,
    pub(crate) projected_run_refs: Vec<StateRecordRef>,
    pub(crate) projected_evidence_observations: Vec<EvidenceObservation>,
    pub(crate) projected_artifacts: Vec<ArtifactRef>,
    pub(crate) projected_required_criterion_ids: Option<BTreeSet<String>>,
    pub(crate) projected_resolved_unrecorded_change_ids: BTreeSet<String>,
    pub(crate) unresolved_unrecorded_changes: Vec<UnrecordedChangeRecord>,
    pub(crate) write_tickets: Option<Vec<StoredWriteTicket>>,
    pub(crate) workflow_policy: Option<ProjectWorkflowPolicy>,
    pub(crate) pending_user_action_authorities: Option<Vec<UserActionAuthority>>,
    pub(crate) resolved_judgment_authorities: Option<Vec<UserActionAuthority>>,
    pub(crate) stored_resolved_judgment_authorities:
        HashMap<JudgmentKind, Vec<UserActionAuthority>>,
    pub(crate) non_current_judgment_refs: HashMap<JudgmentKind, Vec<StateRecordRef>>,
}

pub(crate) fn facts_from_projection(
    task: TaskRecord,
    current_change_unit: Option<ChangeUnitRecord>,
    current_close_basis: Option<CurrentCloseBasis>,
    pending_user_action_refs: Vec<StateRecordRef>,
    blocker_refs: Vec<StateRecordRef>,
    evidence_summary: Option<EvidenceSummary>,
    now: UtcTimestamp,
) -> CloseReadinessFacts {
    let artifact_refs = evidence_summary
        .as_ref()
        .map(|summary| summary.artifact_refs.clone())
        .unwrap_or_default();
    CloseReadinessFacts {
        now,
        task,
        current_change_unit,
        current_close_basis,
        pending_user_action_refs,
        blocker_refs,
        evidence_summary,
        acceptance_criteria: None,
        artifact_refs,
        projected_run_refs: Vec::new(),
        projected_evidence_observations: Vec::new(),
        projected_artifacts: Vec::new(),
        projected_required_criterion_ids: None,
        projected_resolved_unrecorded_change_ids: BTreeSet::new(),
        unresolved_unrecorded_changes: Vec::new(),
        write_tickets: None,
        workflow_policy: None,
        pending_user_action_authorities: None,
        resolved_judgment_authorities: None,
        stored_resolved_judgment_authorities: HashMap::new(),
        non_current_judgment_refs: HashMap::new(),
    }
}

pub(crate) fn facts_with_projected_acceptance_criteria(
    mut facts: CloseReadinessFacts,
    acceptance_criteria: &[volicord_types::schema::AcceptanceCriterion],
) -> CloseReadinessFacts {
    facts.projected_required_criterion_ids =
        Some(required_acceptance_criterion_ids(acceptance_criteria));
    facts.acceptance_criteria = Some(acceptance_criteria.to_vec());
    facts
}

pub(crate) fn facts_with_record_run_projection(
    mut facts: CloseReadinessFacts,
    run_ref: StateRecordRef,
    evidence_observations: Vec<EvidenceObservation>,
    registered_artifacts: Vec<ArtifactRef>,
) -> CloseReadinessFacts {
    facts.projected_run_refs.push(run_ref);
    facts.projected_evidence_observations = evidence_observations;
    facts.projected_artifacts = registered_artifacts;
    facts
}

pub(crate) fn facts_with_pending_authorities(
    mut facts: CloseReadinessFacts,
    authorities: Vec<UserActionAuthority>,
) -> CloseReadinessFacts {
    facts.pending_user_action_authorities = Some(authorities);
    facts
}

pub(crate) fn facts_with_resolved_authorities(
    mut facts: CloseReadinessFacts,
    authorities: Vec<UserActionAuthority>,
) -> CloseReadinessFacts {
    facts.resolved_judgment_authorities = Some(authorities);
    facts
}

pub(crate) fn facts_with_resolved_unrecorded_changes(
    mut facts: CloseReadinessFacts,
    unrecorded_change_ids: impl IntoIterator<Item = String>,
) -> CloseReadinessFacts {
    facts.projected_resolved_unrecorded_change_ids = unrecorded_change_ids.into_iter().collect();
    facts
}

/// Acquires one typed current snapshot for the service entry point.
pub(super) fn acquire_close_readiness_facts(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    project_id: &ProjectId,
    task_id: &TaskId,
    now: &UtcTimestamp,
) -> Result<CloseReadinessFacts, CloseReadinessError> {
    let task = store
        .task_record(task_id)
        .map_err(CorePipelineError::from)?
        .ok_or(CloseReadinessError::NoActiveTask)?;
    let current_change_unit = store
        .current_change_unit(task_id)
        .map_err(CorePipelineError::from)?;
    let task_revision = store
        .task_revision_record(task_id)
        .map_err(CorePipelineError::from)?
        .ok_or(CloseReadinessError::NoActiveTask)?;
    let current_close_basis = task_revision.current_close_basis;
    let pending_user_action_refs = store
        .pending_user_action_refs(task_id, project_state.state_version, now)
        .map_err(CorePipelineError::from)?
        .into_iter()
        .map(state_ref_from_stored)
        .collect::<Vec<_>>();
    let blocker_refs = store
        .active_blocker_refs(task_id, project_state.state_version)
        .map_err(CorePipelineError::from)?
        .into_iter()
        .map(state_ref_from_stored)
        .collect::<Vec<_>>();
    let evidence_record = current_close_basis
        .as_ref()
        .and_then(|basis| basis.evidence_summary_ref.as_ref())
        .map(|evidence_ref| {
            store
                .evidence_summary_record(evidence_ref.record_id.as_str())
                .map_err(CorePipelineError::from)
        })
        .transpose()?
        .flatten();
    let acceptance_criteria = active_acceptance_criteria(store, task_id)?;
    let required_criterion_ids = required_acceptance_criterion_ids(&acceptance_criteria);
    let evidence_facts = load_close_evidence_summary_facts(
        store,
        evidence_record.as_ref(),
        &task,
        project_id,
        task_id,
        project_state.state_version,
    )?;
    let evidence_summary = project_close_evidence_summary(evidence_facts, &required_criterion_ids);
    let artifact_refs = evidence_summary
        .as_ref()
        .map(|summary| summary.artifact_refs.clone())
        .unwrap_or_default();

    let mut facts = CloseReadinessFacts {
        now: now.clone(),
        task,
        current_change_unit,
        current_close_basis,
        pending_user_action_refs,
        blocker_refs,
        evidence_summary,
        acceptance_criteria: Some(acceptance_criteria),
        artifact_refs,
        projected_run_refs: Vec::new(),
        projected_evidence_observations: Vec::new(),
        projected_artifacts: Vec::new(),
        projected_required_criterion_ids: Some(required_criterion_ids),
        projected_resolved_unrecorded_change_ids: BTreeSet::new(),
        unresolved_unrecorded_changes: Vec::new(),
        write_tickets: None,
        workflow_policy: None,
        pending_user_action_authorities: None,
        resolved_judgment_authorities: None,
        stored_resolved_judgment_authorities: HashMap::new(),
        non_current_judgment_refs: HashMap::new(),
    };
    acquire_unrecorded_change_facts(store, &mut facts)?;
    facts.workflow_policy = Some(project_workflow_policy(store).map_err(CorePipelineError::from)?);
    Ok(facts)
}

pub(super) fn acquire_unrecorded_change_facts(
    store: &CoreProjectStore,
    facts: &mut CloseReadinessFacts,
) -> Result<(), CloseReadinessError> {
    let unresolved = store
        .unresolved_unrecorded_changes(None)
        .map_err(CorePipelineError::from)
        .map_err(CloseReadinessError::Core)?;
    facts.unresolved_unrecorded_changes = unresolved
        .into_iter()
        .filter(|record| {
            !facts
                .projected_resolved_unrecorded_change_ids
                .contains(&record.unrecorded_change_id)
        })
        .collect();
    Ok(())
}

pub(super) fn acquire_projected_store_facts(
    store: &CoreProjectStore,
    task_id: &TaskId,
    facts: &mut CloseReadinessFacts,
) -> Result<(), CloseReadinessError> {
    acquire_unrecorded_change_facts(store, facts)?;
    if facts.acceptance_criteria.is_none() {
        let acceptance_criteria = active_acceptance_criteria(store, task_id)?;
        facts.projected_required_criterion_ids =
            Some(required_acceptance_criterion_ids(&acceptance_criteria));
        facts.acceptance_criteria = Some(acceptance_criteria);
    }
    if facts.workflow_policy.is_none() {
        facts.workflow_policy =
            Some(project_workflow_policy(store).map_err(CorePipelineError::from)?);
    }
    Ok(())
}

pub(super) fn required_criteria_for_close_context(
    context: &CloseReadinessFacts,
) -> CoreResult<&BTreeSet<String>> {
    if let Some(required) = context.projected_required_criterion_ids.as_ref() {
        return Ok(required);
    }
    Err(CorePipelineError::InvalidDispatch {
        detail: "close-readiness acceptance criteria were not acquired".to_owned(),
    })
}

pub(super) fn workflow_policy_for_close_context(
    context: &CloseReadinessFacts,
) -> CoreResult<&ProjectWorkflowPolicy> {
    context
        .workflow_policy
        .as_ref()
        .ok_or_else(|| CorePipelineError::InvalidDispatch {
            detail: "close-readiness workflow policy was not acquired".to_owned(),
        })
}

#[cfg(test)]
#[path = "tests/facts.rs"]
mod tests;
