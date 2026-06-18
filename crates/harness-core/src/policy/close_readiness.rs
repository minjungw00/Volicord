use std::collections::BTreeSet;

use harness_types::{
    ActorKind, BaselineRef, ChangeUnitId, CloseReadinessBlocker, CloseReadinessBlockerCategory,
    CurrentCloseBasis, JsonObject, JudgmentBasis, JudgmentBasisCompatibilityStatus, JudgmentKind,
    JudgmentResolutionOutcome, MethodName, NextActionKind, NextActionSummary, ProjectId,
    RecordUserJudgmentPayload, RequiredNullable, RiskAcceptanceCoverage, RiskId, StateRecordKind,
    StateRecordRef, TaskId, UserJudgmentResolution, UserJudgmentStatus,
};
use serde_json::Value;

pub(crate) fn is_terminal_lifecycle(value: &str) -> bool {
    matches!(value, "completed" | "cancelled" | "superseded")
}

pub(crate) fn close_blocker(
    category: CloseReadinessBlockerCategory,
    code: &'static str,
    message: &'static str,
    related_refs: Vec<StateRecordRef>,
    next_actions: Vec<NextActionSummary>,
) -> CloseReadinessBlocker {
    CloseReadinessBlocker {
        category,
        code: code.to_owned(),
        message: message.to_owned(),
        related_refs,
        next_actions,
    }
}

pub(crate) fn close_next_action(
    label: &str,
    required_refs: Vec<StateRecordRef>,
) -> NextActionSummary {
    NextActionSummary {
        action_kind: NextActionKind::CloseTask,
        owner_method: Some(MethodName::CloseTask),
        label: label.to_owned(),
        blocking_question: None,
        required_refs,
    }
}

#[derive(Debug, Clone)]
pub(crate) struct JudgmentAuthority {
    pub(crate) judgment_id: String,
    pub(crate) task_id: TaskId,
    pub(crate) judgment_kind: JudgmentKind,
    pub(crate) status: UserJudgmentStatus,
    pub(crate) resolution_outcome: Option<JudgmentResolutionOutcome>,
    pub(crate) basis_status: JudgmentBasisCompatibilityStatus,
    pub(crate) basis: Option<JudgmentBasis>,
    pub(crate) resolution: Option<UserJudgmentResolution>,
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
    judgment: &JudgmentAuthority,
    requirement: &FinalAcceptanceRequirement<'_>,
) -> bool {
    if !accepted_current_user_authority(judgment, JudgmentKind::FinalAcceptance) {
        return false;
    }
    judgment
        .basis
        .as_ref()
        .is_some_and(|basis| final_acceptance_basis_matches_current(basis, requirement))
}

pub(crate) fn final_acceptance_basis_matches_current(
    basis: &JudgmentBasis,
    requirement: &FinalAcceptanceRequirement<'_>,
) -> bool {
    basis.task_id == *requirement.task_id
        && basis.change_unit_id.as_ref() == Some(requirement.change_unit_id)
        && basis.scope_revision == requirement.scope_revision
        && basis.close_basis_revision.as_ref() == Some(&requirement.close_basis_revision)
        && basis.baseline_ref.as_ref() == requirement.baseline_ref
        && state_refs_match(&basis.result_refs, requirement.result_refs)
}

pub(crate) fn current_residual_risk_acceptance_coverage(
    project_id: &ProjectId,
    task_id: &TaskId,
    state_version: u64,
    current_close_basis: &CurrentCloseBasis,
    judgments: &[JudgmentAuthority],
) -> Vec<RiskAcceptanceCoverage> {
    current_close_basis
        .residual_risks
        .iter()
        .map(|risk| {
            let accepted_by_judgment_refs = if risk.acceptance_required {
                judgments
                    .iter()
                    .filter(|judgment| {
                        current_residual_risk_acceptance_covers(
                            judgment,
                            current_close_basis,
                            &risk.risk_id,
                        )
                    })
                    .map(|judgment| StateRecordRef {
                        record_kind: StateRecordKind::UserJudgment,
                        record_id: harness_types::RecordId::new(judgment.judgment_id.clone()),
                        project_id: project_id.clone(),
                        task_id: Some(task_id.clone()).into(),
                        state_version: Some(state_version).into(),
                    })
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let accepted = !risk.acceptance_required || !accepted_by_judgment_refs.is_empty();
            RiskAcceptanceCoverage {
                risk_id: risk.risk_id.clone(),
                accepted,
                accepted_by_judgment_refs,
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
    judgment: &JudgmentAuthority,
    current_close_basis: &CurrentCloseBasis,
    risk_id: &RiskId,
) -> bool {
    if !accepted_current_user_authority(judgment, JudgmentKind::ResidualRiskAcceptance) {
        return false;
    }
    let Some(basis) = judgment.basis.as_ref() else {
        return false;
    };
    if !residual_risk_basis_matches_current(basis, current_close_basis) {
        return false;
    }
    let Some(resolution) = judgment.resolution.as_ref() else {
        return false;
    };
    let accepted_ids = accepted_risk_ids_from_resolution(resolution);
    accepted_ids.is_subset(&risk_id_set(&basis.residual_risk_ids)) && accepted_ids.contains(risk_id)
}

pub(crate) fn residual_risk_basis_matches_current(
    basis: &JudgmentBasis,
    current_close_basis: &CurrentCloseBasis,
) -> bool {
    let current_required_ids = current_acceptance_required_risk_ids(current_close_basis);
    basis.task_id == current_close_basis.task_id
        && basis.close_basis_revision.as_ref() == Some(&current_close_basis.close_basis_revision)
        && risk_id_set(&basis.residual_risk_ids) == current_required_ids
}

pub(crate) fn accepted_risk_ids_from_answer(
    answer: &RecordUserJudgmentPayload,
    accepted_risks: &[harness_types::AcceptedRiskInput],
) -> BTreeSet<RiskId> {
    let mut ids = accepted_risks
        .iter()
        .filter(|risk| risk.accepted_for_close)
        .map(|risk| risk.risk_id.clone())
        .collect::<BTreeSet<_>>();
    if let Some(answer) = answer.residual_risk_acceptance.as_ref() {
        ids.extend(accepted_risk_ids_from_object(answer));
    }
    ids
}

pub(crate) fn accepted_risk_ids_within_basis(
    answer: &RecordUserJudgmentPayload,
    accepted_risks: &[harness_types::AcceptedRiskInput],
    basis: &JudgmentBasis,
) -> bool {
    accepted_risk_ids_from_answer(answer, accepted_risks)
        .is_subset(&risk_id_set(&basis.residual_risk_ids))
}

pub(crate) fn current_scope_decision(
    judgment: &JudgmentAuthority,
    task_id: &TaskId,
    scope_revision: u64,
    current_change_unit_id: Option<&ChangeUnitId>,
) -> bool {
    if !judgment_has_current_basis(judgment)
        || judgment.status != UserJudgmentStatus::Resolved
        || judgment.judgment_kind != JudgmentKind::ScopeDecision
        || judgment
            .resolution
            .as_ref()
            .is_none_or(|resolution| resolution.answer.scope_decision.is_none())
    {
        return false;
    }
    let Some(basis) = judgment.basis.as_ref() else {
        return false;
    };
    basis.task_id == *task_id
        && basis.scope_revision == scope_revision
        && basis.change_unit_id.as_ref() == current_change_unit_id
}

pub(crate) fn accepted_current_user_authority(
    judgment: &JudgmentAuthority,
    required_kind: JudgmentKind,
) -> bool {
    if !judgment_has_current_basis(judgment)
        || judgment.status != UserJudgmentStatus::Resolved
        || judgment.judgment_kind != required_kind
        || judgment.resolution_outcome != Some(JudgmentResolutionOutcome::Accepted)
    {
        return false;
    }
    let Some(resolution) = judgment.resolution.as_ref() else {
        return false;
    };
    resolution.resolution_outcome == Some(JudgmentResolutionOutcome::Accepted)
        && resolution.resolved_by_actor_kind == ActorKind::User
        && resolution_answer_matches_kind(required_kind, &resolution.answer)
}

pub(crate) fn judgment_has_current_basis(judgment: &JudgmentAuthority) -> bool {
    judgment.basis_status == JudgmentBasisCompatibilityStatus::Current
        && judgment.basis.as_ref().is_some_and(|basis| {
            basis.compatibility_status == JudgmentBasisCompatibilityStatus::Current
        })
}

fn resolution_answer_matches_kind(
    judgment_kind: JudgmentKind,
    answer: &RecordUserJudgmentPayload,
) -> bool {
    match judgment_kind {
        JudgmentKind::ProductDecision => answer.product_decision.is_some(),
        JudgmentKind::TechnicalDecision => answer.technical_decision.is_some(),
        JudgmentKind::ScopeDecision => answer.scope_decision.is_some(),
        JudgmentKind::SensitiveApproval => answer.sensitive_action_scope.is_some(),
        JudgmentKind::FinalAcceptance => answer.final_acceptance.is_some(),
        JudgmentKind::ResidualRiskAcceptance => answer.residual_risk_acceptance.is_some(),
        JudgmentKind::Cancellation => answer.cancellation.is_some(),
    }
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

fn accepted_risk_ids_from_resolution(resolution: &UserJudgmentResolution) -> BTreeSet<RiskId> {
    accepted_risk_ids_from_answer(&resolution.answer, &resolution.accepted_risks)
}

fn accepted_risk_ids_from_object(answer: &JsonObject) -> BTreeSet<RiskId> {
    let mut ids = BTreeSet::new();
    if let Some(value) = answer.get("risk_id").and_then(Value::as_str) {
        ids.insert(RiskId::new(value));
    }
    if let Some(values) = answer.get("risk_ids").and_then(Value::as_array) {
        ids.extend(values.iter().filter_map(Value::as_str).map(RiskId::new));
    }
    ids
}

fn risk_id_set(ids: &[RiskId]) -> BTreeSet<RiskId> {
    ids.iter().cloned().collect()
}

fn state_refs_match(left: &[StateRecordRef], right: &[StateRecordRef]) -> bool {
    let mut left_keys = left.iter().map(state_ref_sort_key).collect::<Vec<_>>();
    let mut right_keys = right.iter().map(state_ref_sort_key).collect::<Vec<_>>();
    left_keys.sort();
    right_keys.sort();
    left_keys == right_keys
}

fn state_ref_sort_key(record_ref: &StateRecordRef) -> String {
    serde_json::to_string(record_ref).unwrap_or_default()
}
