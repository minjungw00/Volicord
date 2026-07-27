use std::collections::BTreeSet;

use volicord_types::ids::{BaselineRef, ChangeUnitId, ProjectId, RiskId, TaskId};
use volicord_types::schema::{
    CurrentCloseBasis, RequiredNullable, RiskAcceptanceCoverage, StateRecordRef, UserActionBasis,
    UserActionResolutionBody,
};
use volicord_types::values::{
    AcceptancePolicy, StateRecordKind, UserActionKind, UserActionRequiredFor, UtcTimestamp,
};
use volicord_user_action_service::{accepted_current_user_authority, UserActionAuthority};

use super::evidence::state_record_ref_identity_key;

pub(crate) fn is_terminal_lifecycle(value: &str) -> bool {
    matches!(value, "completed" | "cancelled" | "superseded")
}

pub(crate) fn close_acceptance_policy_rank(policy: AcceptancePolicy) -> u8 {
    match policy {
        AcceptancePolicy::NotRequired => 0,
        AcceptancePolicy::PolicyDependent => 1,
        AcceptancePolicy::Required => 2,
    }
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
pub(crate) struct ScopeDecisionAuthorityRequirement<'a> {
    pub(crate) task_id: &'a TaskId,
    pub(crate) scope_revision: u64,
    pub(crate) current_change_unit_id: Option<&'a ChangeUnitId>,
    pub(crate) affected_refs: &'a [StateRecordRef],
    pub(crate) now: &'a UtcTimestamp,
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
                                record_id: volicord_types::ids::RecordId::new(
                                    resolution_id.clone(),
                                ),
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
