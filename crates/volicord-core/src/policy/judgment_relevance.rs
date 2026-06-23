use volicord_types::{
    ChangeUnitId, CurrentCloseBasis, JudgmentBasis, JudgmentKind, JudgmentRequiredFor,
    StateRecordKind, StateRecordRef, TaskId, UserJudgmentStatus,
};

use crate::policy::{
    close_readiness::JudgmentAuthority,
    close_readiness::{final_acceptance_basis_matches_current, final_acceptance_requirement},
    close_readiness::{judgment_has_current_basis, residual_risk_basis_matches_current},
    write_authorization::{
        sensitive_action_scope_matches_requirement, SensitiveApprovalRequirement,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum JudgmentOperation {
    ScopeUpdate,
    PrepareWrite,
    RecordRun,
    CloseComplete,
    CloseCancel,
    CloseSupersede,
}

impl JudgmentOperation {
    pub(crate) const fn required_for(self) -> JudgmentRequiredFor {
        match self {
            Self::ScopeUpdate => JudgmentRequiredFor::ScopeUpdate,
            Self::PrepareWrite => JudgmentRequiredFor::PrepareWrite,
            Self::RecordRun => JudgmentRequiredFor::RecordRun,
            Self::CloseComplete => JudgmentRequiredFor::CloseComplete,
            Self::CloseCancel => JudgmentRequiredFor::CloseCancel,
            Self::CloseSupersede => JudgmentRequiredFor::CloseSupersede,
        }
    }
}

pub(crate) struct JudgmentOperationContext<'a> {
    pub(crate) operation: JudgmentOperation,
    pub(crate) task_id: &'a TaskId,
    pub(crate) change_unit_id: Option<&'a ChangeUnitId>,
    pub(crate) scope_revision: u64,
    pub(crate) close_basis: Option<&'a CurrentCloseBasis>,
    pub(crate) operation_refs: &'a [StateRecordRef],
    pub(crate) sensitive_approval: Option<&'a SensitiveApprovalRequirement<'a>>,
}

pub(crate) fn judgment_blocks_operation(
    judgment: &JudgmentAuthority,
    context: &JudgmentOperationContext<'_>,
) -> bool {
    if judgment.status != UserJudgmentStatus::Pending || !judgment_has_current_basis(judgment) {
        return false;
    }
    if !judgment
        .required_for
        .iter()
        .any(|target| *target == context.operation.required_for())
    {
        return false;
    }
    if !judgment_kind_relevant_to_operation(judgment.judgment_kind, context.operation) {
        return false;
    }
    let Some(basis) = judgment.basis.as_ref() else {
        return false;
    };
    if !basis_matches_operation_context(basis, context) {
        return false;
    }
    if !affected_refs_overlap(&judgment.affected_refs, context.operation_refs) {
        return false;
    }
    if judgment.judgment_kind == JudgmentKind::SensitiveApproval {
        let Some(requirement) = context.sensitive_approval else {
            return false;
        };
        let Some(scope) = basis.sensitive_action_scope.as_ref() else {
            return false;
        };
        return sensitive_action_scope_matches_requirement(scope, requirement);
    }
    true
}

pub(crate) fn judgment_required_for(
    judgment: &JudgmentAuthority,
    target: JudgmentRequiredFor,
) -> bool {
    judgment.required_for.contains(&target)
}

fn judgment_kind_relevant_to_operation(
    judgment_kind: JudgmentKind,
    operation: JudgmentOperation,
) -> bool {
    match operation {
        JudgmentOperation::PrepareWrite => matches!(
            judgment_kind,
            JudgmentKind::ProductDecision
                | JudgmentKind::TechnicalDecision
                | JudgmentKind::ScopeDecision
                | JudgmentKind::SensitiveApproval
        ),
        JudgmentOperation::RecordRun => matches!(
            judgment_kind,
            JudgmentKind::ProductDecision
                | JudgmentKind::TechnicalDecision
                | JudgmentKind::ScopeDecision
                | JudgmentKind::SensitiveApproval
        ),
        JudgmentOperation::CloseComplete => matches!(
            judgment_kind,
            JudgmentKind::ProductDecision
                | JudgmentKind::TechnicalDecision
                | JudgmentKind::ScopeDecision
                | JudgmentKind::SensitiveApproval
                | JudgmentKind::FinalAcceptance
                | JudgmentKind::ResidualRiskAcceptance
        ),
        JudgmentOperation::CloseCancel => judgment_kind == JudgmentKind::Cancellation,
        JudgmentOperation::CloseSupersede => matches!(
            judgment_kind,
            JudgmentKind::ProductDecision
                | JudgmentKind::TechnicalDecision
                | JudgmentKind::ScopeDecision
                | JudgmentKind::SensitiveApproval
        ),
        JudgmentOperation::ScopeUpdate => matches!(
            judgment_kind,
            JudgmentKind::ProductDecision
                | JudgmentKind::TechnicalDecision
                | JudgmentKind::ScopeDecision
        ),
    }
}

fn basis_matches_operation_context(
    basis: &JudgmentBasis,
    context: &JudgmentOperationContext<'_>,
) -> bool {
    if basis.task_id != *context.task_id || basis.scope_revision != context.scope_revision {
        return false;
    }
    if let Some(change_unit_id) = context.change_unit_id {
        if basis.change_unit_id.as_ref() != Some(change_unit_id) {
            return false;
        }
    }
    match context.operation {
        JudgmentOperation::CloseComplete => {
            if let Some(close_basis) = context.close_basis {
                let final_requirement = final_acceptance_requirement(close_basis);
                basis.close_basis_revision.is_none()
                    || final_acceptance_basis_matches_current(basis, &final_requirement)
                    || residual_risk_basis_matches_current(basis, close_basis)
            } else {
                true
            }
        }
        JudgmentOperation::CloseCancel => basis.change_unit_id.as_ref() == context.change_unit_id,
        JudgmentOperation::PrepareWrite
        | JudgmentOperation::RecordRun
        | JudgmentOperation::CloseSupersede
        | JudgmentOperation::ScopeUpdate => true,
    }
}

fn affected_refs_overlap(
    judgment_refs: &[StateRecordRef],
    operation_refs: &[StateRecordRef],
) -> bool {
    if judgment_refs.is_empty() || operation_refs.is_empty() {
        return true;
    }
    judgment_refs.iter().any(|judgment_ref| {
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
