use super::blockers::normalize_close_blockers;
use super::facts::{workflow_policy_for_close_context, CloseReadinessFacts};
use super::summary::CloseReadinessAssessment;
use crate::methods::{
    acceptance_policy_storage, change_unit_effect_contract, evidence_summary_for_display,
    parse_acceptance_policy, PlanError,
};
use crate::pipeline::CorePipelineError;
use crate::policy::close_readiness::close_acceptance_policy_rank;
use crate::policy::close_readiness_evidence::evaluate_evidence_gate;
use crate::policy::workflow::{
    acceptance_policy_for_control, parse_task_control_level, resolve_task_control_authority,
};
use volicord_store::core_pipeline::TaskControlLevelUpdate;
use volicord_types::schema::{CloseReadinessBlocker, RiskAcceptanceCoverage};
use volicord_types::values::{ChangeUnitEffectKind, CloseIntent, CloseState, TaskControlLevel};

/// Responsibility-owned evaluations combined in stable close-readiness order.
pub(super) struct CloseReadinessEvaluations {
    pub(super) risk_acceptance_coverage: Vec<RiskAcceptanceCoverage>,
    pub(super) terminal_change_control: Vec<CloseReadinessBlocker>,
    pub(super) terminal_acceptance: Vec<CloseReadinessBlocker>,
    pub(super) completion_scope: Vec<CloseReadinessBlocker>,
    pub(super) completion_authority: Vec<CloseReadinessBlocker>,
    pub(super) completion_basis: Vec<CloseReadinessBlocker>,
    pub(super) completion_evidence: Vec<CloseReadinessBlocker>,
    pub(super) completion_acceptance: Vec<CloseReadinessBlocker>,
    pub(super) unrecorded_changes: Vec<CloseReadinessBlocker>,
}

/// Resolves the current control floor from already-acquired typed facts.
pub(super) fn resolve_control(
    context: &mut CloseReadinessFacts,
) -> Result<Option<TaskControlLevelUpdate>, PlanError> {
    let workflow_policy = workflow_policy_for_close_context(context)?.clone();
    let current_control = parse_task_control_level(&context.task.effective_control_level)
        .map_err(CorePipelineError::from)?;
    let resolved_control = resolve_task_control_authority(&context.task, &workflow_policy)
        .map_err(CorePipelineError::from)?;
    let sensitive_effect = context
        .current_change_unit
        .as_ref()
        .map(change_unit_effect_contract)
        .transpose()?
        .flatten()
        .is_some_and(|contract| {
            !contract.sensitive_action_expectations.is_empty()
                || contract.allowed_effects.iter().any(|effect| {
                    matches!(
                        effect,
                        ChangeUnitEffectKind::SensitiveAction
                            | ChangeUnitEffectKind::ExternalNetwork
                            | ChangeUnitEffectKind::SecretAccess
                    )
                })
        });
    let current_acceptance = parse_acceptance_policy(&context.task.acceptance_policy)?;
    let next_control = if sensitive_effect {
        TaskControlLevel::Sensitive
    } else {
        resolved_control.effective_control_level
    };
    let control_acceptance = acceptance_policy_for_control(next_control, &workflow_policy);
    let next_acceptance = if close_acceptance_policy_rank(resolved_control.acceptance_policy)
        >= close_acceptance_policy_rank(control_acceptance)
    {
        resolved_control.acceptance_policy
    } else {
        control_acceptance
    };
    let acceptance_raised = close_acceptance_policy_rank(next_acceptance)
        > close_acceptance_policy_rank(current_acceptance);
    let control_raised = next_control > current_control;
    Ok((control_raised || acceptance_raised).then(|| {
        let reason = if sensitive_effect && control_raised {
            "Core raised control to `sensitive` for the current Change Unit effect contract."
                .to_owned()
        } else if control_raised {
            resolved_control.control_level_reason.clone()
        } else {
            context.task.control_level_reason.clone()
        };
        context.task.effective_control_level = next_control.as_str().to_owned();
        context.task.control_level_reason = reason.clone();
        if acceptance_raised {
            context.task.acceptance_policy = acceptance_policy_storage(next_acceptance).to_owned();
            context.task.acceptance_policy_reason = if next_control
                == resolved_control.effective_control_level
                && resolved_control.acceptance_raised
            {
                resolved_control.acceptance_policy_reason.clone()
            } else {
                format!(
                    "Effective control `{}` requires final acceptance for the current close basis.",
                    next_control.as_str()
                )
            };
        }
        TaskControlLevelUpdate {
            task_id: context.task.task_id.clone(),
            effective_control_level: next_control.as_str().to_owned(),
            control_level_reason: reason,
            acceptance_policy: acceptance_raised
                .then(|| acceptance_policy_storage(next_acceptance).to_owned()),
            acceptance_policy_reason: acceptance_raised
                .then(|| context.task.acceptance_policy_reason.clone()),
        }
    }))
}

/// Purely combines typed component evaluations into the shared assessment.
pub(super) fn combine(
    intent: CloseIntent,
    current_state_version: u64,
    context: CloseReadinessFacts,
    control_update: Option<TaskControlLevelUpdate>,
    evaluations: CloseReadinessEvaluations,
) -> Result<CloseReadinessAssessment, PlanError> {
    let CloseReadinessEvaluations {
        risk_acceptance_coverage,
        terminal_change_control,
        terminal_acceptance,
        completion_scope,
        completion_authority,
        completion_basis,
        completion_evidence,
        completion_acceptance,
        unrecorded_changes,
    } = evaluations;
    let mut blockers = terminal_change_control;
    blockers.extend(terminal_acceptance);
    if matches!(intent, CloseIntent::Check | CloseIntent::Complete) {
        blockers.extend(completion_scope);
        blockers.extend(completion_authority);
        blockers.extend(completion_basis);
        blockers.extend(completion_evidence);
        blockers.extend(completion_acceptance);
    }
    blockers.extend(unrecorded_changes);
    normalize_close_blockers(&mut blockers, current_state_version);

    let committed_terminal = intent != CloseIntent::Check && blockers.is_empty();
    let response_state_version = if committed_terminal {
        current_state_version + 1
    } else {
        current_state_version
    };
    let close_state = close_state_for_policy(intent, blockers.is_empty());
    let evidence_summary = context
        .evidence_summary
        .clone()
        .map(|summary| evidence_summary_for_display(summary, context.current_close_basis.as_ref()));
    let acceptance_criteria = context.acceptance_criteria.as_deref().ok_or_else(|| {
        PlanError::Core(CorePipelineError::InvalidDispatch {
            detail: "close-readiness acceptance criteria were not acquired".to_owned(),
        })
    })?;
    let evidence_gate =
        evaluate_evidence_gate(acceptance_criteria, evidence_summary.as_ref(), &blockers);

    Ok(CloseReadinessAssessment {
        context,
        control_update,
        risk_acceptance_coverage,
        blockers,
        committed_terminal,
        response_state_version,
        close_state,
        evidence_gate,
    })
}

fn close_state_for_policy(intent: CloseIntent, allowed: bool) -> CloseState {
    if !allowed {
        return CloseState::Blocked;
    }
    match intent {
        CloseIntent::Check => CloseState::Ready,
        CloseIntent::Complete => CloseState::Closed,
        CloseIntent::Cancel => CloseState::Cancelled,
        CloseIntent::Supersede => CloseState::Superseded,
    }
}

#[cfg(test)]
#[path = "tests/policy.rs"]
mod tests;
