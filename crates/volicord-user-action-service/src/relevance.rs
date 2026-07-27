use std::collections::BTreeSet;
use volicord_types::ids::{BaselineRef, ChangeUnitId, RiskId, TaskId};
use volicord_types::product_path::{parse_product_paths, path_is_within};
use volicord_types::schema::{
    CurrentCloseBasis, SensitiveActionScope, StateRecordRef, UserActionBasis,
};
use volicord_types::values::{
    ActorSource, JudgmentResolutionOutcome, StateRecordKind, UserActionBasisStatus, UserActionKind,
    UserActionOptionAction, UserActionRequiredFor, UserActionStatus, UtcTimestamp,
};

use crate::model::UserActionAuthority;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserActionOperation {
    ScopeUpdate,
    PrepareWrite,
    RecordRun,
    CloseComplete,
    CloseSupersede,
}

impl UserActionOperation {
    pub const fn required_for(self) -> UserActionRequiredFor {
        match self {
            Self::ScopeUpdate => UserActionRequiredFor::ScopeUpdate,
            Self::PrepareWrite => UserActionRequiredFor::PrepareWrite,
            Self::RecordRun => UserActionRequiredFor::RecordRun,
            Self::CloseComplete => UserActionRequiredFor::CloseComplete,
            Self::CloseSupersede => UserActionRequiredFor::CloseSupersede,
        }
    }
}

pub struct UserActionOperationContext<'a> {
    pub operation: UserActionOperation,
    pub task_id: &'a TaskId,
    pub change_unit_id: Option<&'a ChangeUnitId>,
    pub scope_revision: u64,
    pub close_basis: Option<&'a CurrentCloseBasis>,
    pub operation_refs: &'a [StateRecordRef],
    pub sensitive_approval: Option<&'a SensitiveApprovalRequirement<'a>>,
}

/// Current Task coordinates required by cancellation authority.
pub struct CancellationAuthorityRequirement<'a> {
    pub task_id: &'a TaskId,
    pub change_unit_id: Option<&'a ChangeUnitId>,
    pub scope_revision: u64,
}

pub fn user_action_blocks_operation(
    user_action: &UserActionAuthority,
    context: &UserActionOperationContext<'_>,
) -> bool {
    if user_action.status != UserActionStatus::Pending
        || !user_action_has_current_basis(user_action)
    {
        return false;
    }
    if !user_action
        .required_for
        .iter()
        .any(|target| *target == context.operation.required_for())
    {
        return false;
    }
    if !user_action_kind_relevant_to_operation(user_action.action_kind, context.operation) {
        return false;
    }
    let Some(basis) = user_action.basis.as_ref() else {
        return false;
    };
    if !basis_matches_operation_context(basis, context) {
        return false;
    }
    if !affected_refs_overlap(&user_action.affected_refs, context.operation_refs) {
        return false;
    }
    if user_action.action_kind == UserActionKind::SensitiveApproval {
        let Some(requirement) = context.sensitive_approval else {
            return false;
        };
        if basis.coordinates().baseline_ref.as_ref() != requirement.baseline_ref {
            return false;
        }
        let Some(scope) = basis.sensitive_action_scope() else {
            return false;
        };
        return sensitive_action_scope_matches_requirement(scope, requirement);
    }
    true
}

pub fn user_action_required_for(
    user_action: &UserActionAuthority,
    target: UserActionRequiredFor,
) -> bool {
    user_action.required_for.contains(&target)
}

/// Returns whether one resolved UserAction currently authorizes cancellation.
pub fn current_cancellation_authority(
    user_action: &UserActionAuthority,
    requirement: &CancellationAuthorityRequirement<'_>,
) -> bool {
    if !accepted_current_user_authority(user_action, UserActionKind::Cancellation)
        || !user_action
            .required_for
            .contains(&UserActionRequiredFor::CloseCancel)
    {
        return false;
    }
    user_action.basis.as_ref().is_some_and(|basis| {
        let coordinates = basis.coordinates();
        coordinates.task_id == *requirement.task_id
            && coordinates.scope_revision == requirement.scope_revision
            && coordinates.change_unit_id.as_ref() == requirement.change_unit_id
    })
}

pub fn user_action_keeps_task_waiting(
    user_action: &UserActionAuthority,
    task_id: &TaskId,
    current_change_unit_id: Option<&ChangeUnitId>,
    scope_revision: u64,
) -> bool {
    if user_action.status != UserActionStatus::Pending
        || !user_action_has_current_basis(user_action)
        || user_action.task_id != *task_id
        || !user_action
            .required_for
            .iter()
            .any(|target| *target != UserActionRequiredFor::Informational)
    {
        return false;
    }
    user_action.basis.as_ref().is_some_and(|basis| {
        let coordinates = basis.coordinates();
        coordinates.task_id == *task_id
            && coordinates.scope_revision == scope_revision
            && coordinates.change_unit_id.as_ref() == current_change_unit_id
    })
}

fn user_action_kind_relevant_to_operation(
    action_kind: UserActionKind,
    operation: UserActionOperation,
) -> bool {
    action_kind.is_compatible_with_required_for(operation.required_for())
}

fn basis_matches_operation_context(
    basis: &UserActionBasis,
    context: &UserActionOperationContext<'_>,
) -> bool {
    let coordinates = basis.coordinates();
    if coordinates.task_id != *context.task_id
        || coordinates.scope_revision != context.scope_revision
    {
        return false;
    }
    if let Some(change_unit_id) = context.change_unit_id {
        if coordinates.change_unit_id.as_ref() != Some(change_unit_id) {
            return false;
        }
    }
    match context.operation {
        UserActionOperation::CloseComplete => {
            if let Some(close_basis) = context.close_basis {
                basis.close_basis_revision().is_none()
                    || final_acceptance_basis_matches_current(basis, close_basis)
                    || residual_risk_basis_matches_current(basis, close_basis)
            } else {
                true
            }
        }
        UserActionOperation::PrepareWrite
        | UserActionOperation::RecordRun
        | UserActionOperation::CloseSupersede
        | UserActionOperation::ScopeUpdate => true,
    }
}

/// Semantic facts required to match one sensitive-action approval.
pub struct SensitiveApprovalRequirement<'a> {
    pub task_id: &'a TaskId,
    pub change_unit_id: &'a ChangeUnitId,
    pub scope_revision: u64,
    pub operation: &'a str,
    pub normalized_paths: &'a [String],
    pub sensitive_categories: &'a [String],
    pub baseline_ref: Option<&'a BaselineRef>,
    pub required_for: UserActionRequiredFor,
    pub now: &'a UtcTimestamp,
}

pub fn sensitive_action_scope_matches_requirement(
    scope: &SensitiveActionScope,
    requirement: &SensitiveApprovalRequirement<'_>,
) -> bool {
    if scope
        .expires_at
        .as_ref()
        .is_some_and(|expires_at| requirement.now >= expires_at)
        || scope.action_kind != requirement.operation.trim()
    {
        return false;
    }
    let approved_categories = scope
        .sensitive_categories
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if !requirement
        .sensitive_categories
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        .is_subset(&approved_categories)
    {
        return false;
    }
    let Ok(approved_paths) = parse_product_paths(&scope.intended_paths) else {
        return false;
    };
    requirement.normalized_paths.iter().all(|path| {
        approved_paths
            .iter()
            .any(|approved| path_is_within(path, approved.as_str()))
    })
}

pub fn current_sensitive_approval(
    user_action: &UserActionAuthority,
    requirement: &SensitiveApprovalRequirement<'_>,
) -> bool {
    if !accepted_current_user_authority(user_action, UserActionKind::SensitiveApproval)
        || !user_action.required_for.contains(&requirement.required_for)
    {
        return false;
    }
    let Some(basis) = user_action.basis.as_ref() else {
        return false;
    };
    let coordinates = basis.coordinates();
    if coordinates.task_id != *requirement.task_id
        || coordinates.change_unit_id.as_ref() != Some(requirement.change_unit_id)
        || coordinates.scope_revision != requirement.scope_revision
        || coordinates.baseline_ref.as_ref() != requirement.baseline_ref
    {
        return false;
    }
    basis
        .sensitive_action_scope()
        .is_some_and(|scope| sensitive_action_scope_matches_requirement(scope, requirement))
}

pub fn user_action_has_current_basis(user_action: &UserActionAuthority) -> bool {
    user_action.basis_status == UserActionBasisStatus::Current
        && user_action
            .basis
            .as_ref()
            .is_some_and(|basis| basis.compatibility_status() == UserActionBasisStatus::Current)
}

pub fn accepted_current_user_authority(
    user_action: &UserActionAuthority,
    required_kind: UserActionKind,
) -> bool {
    if !user_action_has_current_basis(user_action)
        || user_action.status != UserActionStatus::Resolved
        || user_action.action_kind != required_kind
        || user_action.machine_action != Some(UserActionOptionAction::Accept)
        || user_action.resolution_outcome != Some(JudgmentResolutionOutcome::Accepted)
    {
        return false;
    }
    matches!(
        user_action.resolution.as_ref(),
        Some(volicord_types::schema::UserActionResolutionBody::Choice {
            machine_action: UserActionOptionAction::Accept,
            resolution_outcome: JudgmentResolutionOutcome::Accepted,
            ..
        })
    ) && verified_user_channel_provenance(user_action)
}

pub fn verified_user_channel_provenance(user_action: &UserActionAuthority) -> bool {
    user_action.resolved_by_actor_source == Some(ActorSource::LocalUser)
        && user_action.resolved_verification_basis.is_some()
        && user_action
            .resolved_assurance_level
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
}

fn final_acceptance_basis_matches_current(
    basis: &UserActionBasis,
    current: &CurrentCloseBasis,
) -> bool {
    let coordinates = basis.coordinates();
    coordinates.task_id == current.task_id
        && coordinates.change_unit_id.as_ref() == Some(&current.change_unit_id)
        && coordinates.scope_revision == current.scope_revision
        && basis.close_basis_revision() == Some(current.close_basis_revision)
        && coordinates.baseline_ref.as_ref() == current.baseline_ref.as_ref()
        && state_refs_match(basis.result_refs(), &current.result_refs)
}

fn residual_risk_basis_matches_current(
    basis: &UserActionBasis,
    current: &CurrentCloseBasis,
) -> bool {
    let required = current
        .residual_risks
        .iter()
        .filter(|risk| risk.acceptance_required)
        .map(|risk| risk.risk_id.clone())
        .collect::<BTreeSet<RiskId>>();
    basis.coordinates().task_id == current.task_id
        && basis.close_basis_revision() == Some(current.close_basis_revision)
        && basis
            .residual_risk_ids()
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            == required
}

fn state_refs_match(left: &[StateRecordRef], right: &[StateRecordRef]) -> bool {
    let mut left = left.iter().map(state_ref_key).collect::<Vec<_>>();
    let mut right = right.iter().map(state_ref_key).collect::<Vec<_>>();
    left.sort();
    right.sort();
    left == right
}

fn state_ref_key(reference: &StateRecordRef) -> (String, String, String, Option<String>) {
    (
        format!("{:?}", reference.record_kind),
        reference.record_id.as_str().to_owned(),
        reference.project_id.as_str().to_owned(),
        reference.task_id.as_ref().map(|id| id.as_str().to_owned()),
    )
}

fn affected_refs_overlap(
    user_action_refs: &[StateRecordRef],
    operation_refs: &[StateRecordRef],
) -> bool {
    if user_action_refs.is_empty() || operation_refs.is_empty() {
        return true;
    }
    user_action_refs.iter().any(|judgment_ref| {
        operation_refs
            .iter()
            .any(|op_ref| refs_overlap(judgment_ref, op_ref))
    })
}

fn refs_overlap(left: &StateRecordRef, right: &StateRecordRef) -> bool {
    if left.project_id != right.project_id {
        return false;
    }
    if left.record_kind == StateRecordKind::Task && right.task_id.as_ref() == left.task_id.as_ref()
    {
        return true;
    }
    if right.record_kind == StateRecordKind::Task && left.task_id.as_ref() == right.task_id.as_ref()
    {
        return true;
    }
    left.record_kind == right.record_kind && left.record_id == right.record_id
}
