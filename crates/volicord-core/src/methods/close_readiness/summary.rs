use super::facts::CloseReadinessFacts;
use volicord_store::core_pipeline::TaskControlLevelUpdate;
use volicord_types::schema::{
    CloseReadinessBlocker, CurrentCloseBasis, EvidenceGateSummary, RiskAcceptanceCoverage,
};
use volicord_types::values::CloseState;

/// Complete close-operation evaluation consumed only by `close_task`.
pub(crate) struct CloseReadinessAssessment {
    pub(crate) context: CloseReadinessFacts,
    pub(crate) control_update: Option<TaskControlLevelUpdate>,
    pub(crate) risk_acceptance_coverage: Vec<RiskAcceptanceCoverage>,
    pub(crate) blockers: Vec<CloseReadinessBlocker>,
    pub(crate) committed_terminal: bool,
    pub(crate) response_state_version: u64,
    pub(crate) close_state: CloseState,
    pub(crate) evidence_gate: EvidenceGateSummary,
}

/// Deliberate method-neutral projection consumed by read and sibling planners.
pub(crate) struct CloseReadinessSummary {
    pub(crate) close_state: CloseState,
    pub(crate) current_close_basis: Option<CurrentCloseBasis>,
    pub(crate) risk_acceptance_coverage: Vec<RiskAcceptanceCoverage>,
    pub(crate) blockers: Vec<CloseReadinessBlocker>,
    pub(crate) evidence_gate: EvidenceGateSummary,
}

impl From<CloseReadinessAssessment> for CloseReadinessSummary {
    fn from(assessment: CloseReadinessAssessment) -> Self {
        Self {
            close_state: assessment.close_state,
            current_close_basis: assessment.context.current_close_basis,
            risk_acceptance_coverage: assessment.risk_acceptance_coverage,
            blockers: assessment.blockers,
            evidence_gate: assessment.evidence_gate,
        }
    }
}
