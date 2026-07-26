use volicord_types::values::{JudgmentKind, JudgmentResolutionOutcome, ProjectContinuityKind};

pub(crate) fn judgment_continuity_kind(
    judgment_kind: JudgmentKind,
    outcome: JudgmentResolutionOutcome,
) -> Option<ProjectContinuityKind> {
    if outcome != JudgmentResolutionOutcome::Accepted {
        return None;
    }
    match judgment_kind {
        JudgmentKind::ProductDecision
        | JudgmentKind::TechnicalDecision
        | JudgmentKind::ScopeDecision => Some(ProjectContinuityKind::Decision),
        JudgmentKind::ResidualRiskAcceptance => Some(ProjectContinuityKind::AcceptedRisk),
        JudgmentKind::SensitiveApproval
        | JudgmentKind::FinalAcceptance
        | JudgmentKind::Cancellation => None,
    }
}

pub(crate) fn decision_title_prefix(judgment_kind: JudgmentKind) -> &'static str {
    match judgment_kind {
        JudgmentKind::ProductDecision => "Product decision",
        JudgmentKind::TechnicalDecision => "Technical decision",
        JudgmentKind::ScopeDecision => "Scope decision",
        JudgmentKind::ResidualRiskAcceptance => "Residual risk acceptance",
        JudgmentKind::SensitiveApproval => "Sensitive approval",
        JudgmentKind::FinalAcceptance => "Final acceptance",
        JudgmentKind::Cancellation => "Cancellation",
    }
}
