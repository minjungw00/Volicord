use volicord_types::{
    ChangeUnitId, CurrentCloseBasis, StateRecordKind, StateRecordRef, TaskId, UserActionBasis,
    UserActionKind, UserActionRequiredFor, UserActionStatus,
};

use crate::policy::{
    close_readiness::UserActionAuthority,
    close_readiness::{final_acceptance_basis_matches_current, final_acceptance_requirement},
    close_readiness::{residual_risk_basis_matches_current, user_action_has_current_basis},
    write_ticket::{sensitive_action_scope_matches_requirement, SensitiveApprovalRequirement},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UserActionOperation {
    ScopeUpdate,
    PrepareWrite,
    RecordRun,
    CloseComplete,
    CloseSupersede,
}

impl UserActionOperation {
    pub(crate) const fn required_for(self) -> UserActionRequiredFor {
        match self {
            Self::ScopeUpdate => UserActionRequiredFor::ScopeUpdate,
            Self::PrepareWrite => UserActionRequiredFor::PrepareWrite,
            Self::RecordRun => UserActionRequiredFor::RecordRun,
            Self::CloseComplete => UserActionRequiredFor::CloseComplete,
            Self::CloseSupersede => UserActionRequiredFor::CloseSupersede,
        }
    }
}

pub(crate) struct UserActionOperationContext<'a> {
    pub(crate) operation: UserActionOperation,
    pub(crate) task_id: &'a TaskId,
    pub(crate) change_unit_id: Option<&'a ChangeUnitId>,
    pub(crate) scope_revision: u64,
    pub(crate) close_basis: Option<&'a CurrentCloseBasis>,
    pub(crate) operation_refs: &'a [StateRecordRef],
    pub(crate) sensitive_approval: Option<&'a SensitiveApprovalRequirement<'a>>,
}

pub(crate) fn user_action_blocks_operation(
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

pub(crate) fn user_action_required_for(
    user_action: &UserActionAuthority,
    target: UserActionRequiredFor,
) -> bool {
    user_action.required_for.contains(&target)
}

pub(crate) fn user_action_keeps_task_waiting(
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
                let final_requirement = final_acceptance_requirement(close_basis);
                basis.close_basis_revision().is_none()
                    || final_acceptance_basis_matches_current(basis, &final_requirement)
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
