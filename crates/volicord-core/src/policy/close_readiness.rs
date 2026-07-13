use std::collections::BTreeSet;

use volicord_store::core_pipeline::RunRecord;
use volicord_types::{
    ActorSource, BaselineRef, ChangeUnitId, CloseReadinessBlocker, CloseReadinessBlockerCategory,
    CurrentCloseBasis, JudgmentResolutionOutcome, MethodName, NextActionKind,
    NextActionPresentationRole, NextActionSummary, OperationCategory, ProjectId, RequiredNullable,
    RiskAcceptanceCoverage, RiskId, StateRecordKind, StateRecordRef, TaskId, UserActionBasis,
    UserActionBasisStatus, UserActionKind, UserActionOptionAction, UserActionRequiredFor,
    UserActionResolutionBody, UserActionStatus, UtcTimestamp,
};

use super::evidence::state_record_ref_identity_key;

pub(crate) fn is_terminal_lifecycle(value: &str) -> bool {
    matches!(value, "completed" | "cancelled" | "superseded")
}

pub(crate) fn close_blocker(
    category: CloseReadinessBlockerCategory,
    code: &'static str,
    message: impl Into<String>,
    related_refs: Vec<StateRecordRef>,
    next_actions: Vec<NextActionSummary>,
) -> CloseReadinessBlocker {
    close_blocker_with_resolution(
        category,
        code,
        message,
        false,
        false,
        related_refs,
        next_actions,
    )
}

pub(crate) fn close_blocker_with_resolution(
    category: CloseReadinessBlockerCategory,
    code: &'static str,
    message: impl Into<String>,
    can_resolve_in_chat: bool,
    outside_chat_action_required: bool,
    related_refs: Vec<StateRecordRef>,
    next_actions: Vec<NextActionSummary>,
) -> CloseReadinessBlocker {
    CloseReadinessBlocker {
        category,
        code: code.to_owned(),
        message: message.into(),
        control_surface: None,
        can_resolve_in_chat,
        outside_chat_action_required,
        related_refs,
        next_actions,
    }
}

pub(crate) fn close_next_action(
    label: &str,
    required_refs: Vec<StateRecordRef>,
) -> NextActionSummary {
    NextActionSummary {
        presentation_role: NextActionPresentationRole::Primary,
        action_kind: NextActionKind::CloseTask,
        owner_method: Some(MethodName::CloseTask),
        allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
        label: label.to_owned(),
        blocking_question: None,
        expected_state_version: RequiredNullable::null(),
        required_refs,
    }
}

#[derive(Debug, Clone)]
pub(crate) struct UserActionAuthority {
    pub(crate) user_action_request_id: String,
    pub(crate) user_action_resolution_id: Option<String>,
    pub(crate) task_id: TaskId,
    pub(crate) action_kind: UserActionKind,
    pub(crate) status: UserActionStatus,
    pub(crate) required_for: Vec<UserActionRequiredFor>,
    pub(crate) affected_refs: Vec<StateRecordRef>,
    pub(crate) machine_action: Option<UserActionOptionAction>,
    pub(crate) resolution_outcome: Option<JudgmentResolutionOutcome>,
    pub(crate) resolved_by_actor_source: Option<ActorSource>,
    pub(crate) resolved_verification_basis: Option<String>,
    pub(crate) resolved_assurance_level: Option<String>,
    pub(crate) basis_status: UserActionBasisStatus,
    pub(crate) basis: Option<UserActionBasis>,
    pub(crate) resolution: Option<UserActionResolutionBody>,
    pub(crate) expires_at: Option<UtcTimestamp>,
}

#[derive(Debug, Clone)]
pub(crate) struct FinalAcceptanceRequirement<'a> {
    pub(crate) task_id: &'a TaskId,
    pub(crate) change_unit_id: &'a ChangeUnitId,
    pub(crate) scope_revision: u64,
    pub(crate) close_basis_revision: u64,
    pub(crate) baseline_ref: Option<&'a BaselineRef>,
    pub(crate) result_refs: &'a [StateRecordRef],
}

#[derive(Debug, Clone)]
pub(crate) struct CancellationAuthorityRequirement<'a> {
    pub(crate) task_id: &'a TaskId,
    pub(crate) change_unit_id: Option<&'a ChangeUnitId>,
    pub(crate) scope_revision: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct ScopeDecisionAuthorityRequirement<'a> {
    pub(crate) task_id: &'a TaskId,
    pub(crate) scope_revision: u64,
    pub(crate) current_change_unit_id: Option<&'a ChangeUnitId>,
    pub(crate) affected_refs: &'a [StateRecordRef],
    pub(crate) now: &'a UtcTimestamp,
}

pub(crate) fn close_basis_is_current(
    basis: &CurrentCloseBasis,
    task_id: &TaskId,
    current_change_unit_id: Option<&str>,
    scope_revision: u64,
    close_basis_revision: u64,
    baseline_ref: Option<&str>,
) -> bool {
    basis.task_id == *task_id
        && current_change_unit_id == Some(basis.change_unit_id.as_str())
        && basis.scope_revision == scope_revision
        && basis.close_basis_revision == close_basis_revision
        && basis.baseline_ref.as_ref().map(BaselineRef::as_str) == baseline_ref
}

pub(crate) fn close_basis_run_refs(basis: &CurrentCloseBasis) -> Vec<&StateRecordRef> {
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

pub(crate) fn run_record_matches_close_basis_context(
    record: &RunRecord,
    project_id: &ProjectId,
    task_id: &TaskId,
    change_unit_id: &str,
    scope_revision: u64,
    baseline_ref: Option<&str>,
) -> bool {
    record.project_id == project_id.as_str()
        && record.task_id == task_id.as_str()
        && record.change_unit_id.as_deref() == Some(change_unit_id)
        && record.scope_revision == scope_revision
        && record.baseline_ref.as_deref() == baseline_ref
        && record.status == "recorded"
}

pub(crate) fn final_acceptance_requirement(
    basis: &CurrentCloseBasis,
) -> FinalAcceptanceRequirement<'_> {
    FinalAcceptanceRequirement {
        task_id: &basis.task_id,
        change_unit_id: &basis.change_unit_id,
        scope_revision: basis.scope_revision,
        close_basis_revision: basis.close_basis_revision,
        baseline_ref: basis.baseline_ref.as_ref(),
        result_refs: &basis.result_refs,
    }
}

pub(crate) fn current_final_acceptance(
    judgment: &UserActionAuthority,
    requirement: &FinalAcceptanceRequirement<'_>,
) -> bool {
    if !accepted_current_user_authority(judgment, UserActionKind::FinalAcceptance) {
        return false;
    }
    if !judgment
        .required_for
        .contains(&UserActionRequiredFor::CloseComplete)
    {
        return false;
    }
    judgment
        .basis
        .as_ref()
        .is_some_and(|basis| final_acceptance_basis_matches_current(basis, requirement))
}

pub(crate) fn current_cancellation_authority(
    judgment: &UserActionAuthority,
    requirement: &CancellationAuthorityRequirement<'_>,
) -> bool {
    if !accepted_current_user_authority(judgment, UserActionKind::Cancellation) {
        return false;
    }
    if !judgment
        .required_for
        .contains(&UserActionRequiredFor::CloseCancel)
    {
        return false;
    }
    judgment.basis.as_ref().is_some_and(|basis| {
        let coordinates = basis.coordinates();
        coordinates.task_id == *requirement.task_id
            && coordinates.scope_revision == requirement.scope_revision
            && coordinates.change_unit_id.as_ref() == requirement.change_unit_id
    })
}

pub(crate) fn final_acceptance_basis_matches_current(
    basis: &UserActionBasis,
    requirement: &FinalAcceptanceRequirement<'_>,
) -> bool {
    let coordinates = basis.coordinates();
    coordinates.task_id == *requirement.task_id
        && coordinates.change_unit_id.as_ref() == Some(requirement.change_unit_id)
        && coordinates.scope_revision == requirement.scope_revision
        && basis.close_basis_revision() == Some(requirement.close_basis_revision)
        && coordinates.baseline_ref.as_ref() == requirement.baseline_ref
        && state_refs_match(basis.result_refs(), requirement.result_refs)
}

pub(crate) fn current_residual_risk_acceptance_coverage(
    project_id: &ProjectId,
    task_id: &TaskId,
    state_version: u64,
    current_close_basis: &CurrentCloseBasis,
    user_actions: &[UserActionAuthority],
) -> Vec<RiskAcceptanceCoverage> {
    current_close_basis
        .residual_risks
        .iter()
        .map(|risk| {
            let accepted_by_user_action_resolution_refs = if risk.acceptance_required {
                user_actions
                    .iter()
                    .filter(|user_action| {
                        current_residual_risk_acceptance_covers(
                            user_action,
                            current_close_basis,
                            &risk.risk_id,
                        )
                    })
                    .filter_map(|user_action| {
                        user_action
                            .user_action_resolution_id
                            .as_ref()
                            .map(|resolution_id| StateRecordRef {
                                record_kind: StateRecordKind::UserActionResolution,
                                record_id: volicord_types::RecordId::new(resolution_id.clone()),
                                project_id: project_id.clone(),
                                task_id: Some(task_id.clone()).into(),
                                produced_at_state_version: Some(state_version).into(),
                            })
                    })
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let accepted =
                !risk.acceptance_required || !accepted_by_user_action_resolution_refs.is_empty();
            RiskAcceptanceCoverage {
                risk_id: risk.risk_id.clone(),
                accepted,
                accepted_by_user_action_resolution_refs,
                missing_reason: if accepted {
                    RequiredNullable::null()
                } else {
                    Some("acceptance_required".to_owned()).into()
                },
            }
        })
        .collect()
}

pub(crate) fn current_residual_risk_acceptance_covers(
    user_action: &UserActionAuthority,
    current_close_basis: &CurrentCloseBasis,
    risk_id: &RiskId,
) -> bool {
    if !accepted_current_user_authority(user_action, UserActionKind::ResidualRiskAcceptance) {
        return false;
    }
    if !user_action
        .required_for
        .contains(&UserActionRequiredFor::CloseComplete)
    {
        return false;
    }
    let Some(basis) = user_action.basis.as_ref() else {
        return false;
    };
    if !residual_risk_basis_matches_current(basis, current_close_basis) {
        return false;
    }
    let Some(resolution) = user_action.resolution.as_ref() else {
        return false;
    };
    let accepted_ids = accepted_risk_ids_from_resolution(resolution);
    accepted_ids.is_subset(&risk_id_set(basis.residual_risk_ids()))
        && accepted_ids.contains(risk_id)
}

pub(crate) fn residual_risk_basis_matches_current(
    basis: &UserActionBasis,
    current_close_basis: &CurrentCloseBasis,
) -> bool {
    let current_required_ids = current_acceptance_required_risk_ids(current_close_basis);
    basis.coordinates().task_id == current_close_basis.task_id
        && basis.close_basis_revision() == Some(current_close_basis.close_basis_revision)
        && risk_id_set(basis.residual_risk_ids()) == current_required_ids
}

pub(crate) fn accepted_current_scope_decision_authority(
    judgment: &UserActionAuthority,
    requirement: &ScopeDecisionAuthorityRequirement<'_>,
) -> bool {
    if !accepted_current_user_authority(judgment, UserActionKind::ScopeDecision)
        || !judgment
            .required_for
            .contains(&UserActionRequiredFor::ScopeUpdate)
        || judgment
            .expires_at
            .as_ref()
            .is_some_and(|expires_at| requirement.now >= expires_at)
    {
        return false;
    }
    let Some(basis) = judgment.basis.as_ref() else {
        return false;
    };
    let coordinates = basis.coordinates();
    coordinates.task_id == *requirement.task_id
        && coordinates.scope_revision == requirement.scope_revision
        && coordinates.change_unit_id.as_ref() == requirement.current_change_unit_id
        && scope_decision_refs_are_compatible(
            &judgment.affected_refs,
            requirement.affected_refs,
            requirement.task_id,
        )
}

pub(crate) fn accepted_current_user_authority(
    judgment: &UserActionAuthority,
    required_kind: UserActionKind,
) -> bool {
    if !user_action_has_current_basis(judgment)
        || judgment.status != UserActionStatus::Resolved
        || judgment.action_kind != required_kind
        || judgment.machine_action != Some(UserActionOptionAction::Accept)
        || judgment.resolution_outcome != Some(JudgmentResolutionOutcome::Accepted)
    {
        return false;
    }
    let Some(resolution) = judgment.resolution.as_ref() else {
        return false;
    };
    matches!(
        resolution,
        UserActionResolutionBody::Choice {
            machine_action: UserActionOptionAction::Accept,
            resolution_outcome: JudgmentResolutionOutcome::Accepted,
            ..
        }
    ) && verified_user_channel_provenance(judgment)
}

pub(crate) fn verified_user_channel_provenance(judgment: &UserActionAuthority) -> bool {
    judgment.resolved_by_actor_source == Some(ActorSource::LocalUser)
        && judgment
            .resolved_verification_basis
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && judgment
            .resolved_assurance_level
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
}

pub(crate) fn user_action_has_current_basis(user_action: &UserActionAuthority) -> bool {
    user_action.basis_status == UserActionBasisStatus::Current
        && user_action
            .basis
            .as_ref()
            .is_some_and(|basis| basis.compatibility_status() == UserActionBasisStatus::Current)
}

pub(crate) fn current_acceptance_required_risk_ids(
    current_close_basis: &CurrentCloseBasis,
) -> BTreeSet<RiskId> {
    current_close_basis
        .residual_risks
        .iter()
        .filter(|risk| risk.acceptance_required)
        .map(|risk| risk.risk_id.clone())
        .collect()
}

fn accepted_risk_ids_from_resolution(resolution: &UserActionResolutionBody) -> BTreeSet<RiskId> {
    match resolution {
        UserActionResolutionBody::Choice {
            accepted_risk_ids, ..
        } => accepted_risk_ids.iter().cloned().collect(),
        UserActionResolutionBody::EvidenceObservation { .. } => BTreeSet::new(),
    }
}

fn risk_id_set(ids: &[RiskId]) -> BTreeSet<RiskId> {
    ids.iter().cloned().collect()
}

fn state_refs_match(left: &[StateRecordRef], right: &[StateRecordRef]) -> bool {
    let mut left_keys = left
        .iter()
        .map(state_record_ref_identity_key)
        .collect::<Vec<_>>();
    let mut right_keys = right
        .iter()
        .map(state_record_ref_identity_key)
        .collect::<Vec<_>>();
    left_keys.sort();
    right_keys.sort();
    left_keys == right_keys
}

fn scope_decision_refs_are_compatible(
    judgment_refs: &[StateRecordRef],
    transition_refs: &[StateRecordRef],
    task_id: &TaskId,
) -> bool {
    let Some(first_transition_ref) = transition_refs.first() else {
        return judgment_refs.is_empty();
    };
    if judgment_refs.iter().any(|record_ref| {
        record_ref.project_id != first_transition_ref.project_id
            || !record_ref_task_matches(record_ref, task_id)
    }) {
        return false;
    }
    judgment_refs.is_empty()
        || judgment_refs.iter().any(|judgment_ref| {
            transition_refs
                .iter()
                .any(|transition_ref| state_refs_overlap(judgment_ref, transition_ref))
        })
}

fn record_ref_task_matches(record_ref: &StateRecordRef, task_id: &TaskId) -> bool {
    if record_ref.record_kind == StateRecordKind::Task
        && record_ref.record_id.as_str() != task_id.as_str()
    {
        return false;
    }
    record_ref
        .task_id
        .as_ref()
        .is_none_or(|record_task_id| record_task_id == task_id)
}

fn state_refs_overlap(left: &StateRecordRef, right: &StateRecordRef) -> bool {
    if left.project_id != right.project_id {
        return false;
    }
    if left.record_kind == StateRecordKind::Task && right.task_id.as_ref() == left.task_id.as_ref()
    {
        return true;
    }
    left.record_kind == right.record_kind && left.record_id == right.record_id
}
