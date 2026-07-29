use volicord_types::schema::{CurrentCloseBasis, EvidenceSummary};
use volicord_types::values::EvidenceDisplayState;

pub(crate) fn evidence_summary_for_display(
    mut summary: EvidenceSummary,
    close_basis: Option<&CurrentCloseBasis>,
) -> EvidenceSummary {
    summary.evidence_state = if close_basis
        .and_then(|basis| basis.evidence_summary_ref.as_ref())
        .is_some()
    {
        Some(EvidenceDisplayState::AcceptedForClose)
    } else if evidence_summary_has_attached_evidence(&summary) {
        Some(EvidenceDisplayState::Attached)
    } else {
        None
    };
    summary
}

fn evidence_summary_has_attached_evidence(summary: &EvidenceSummary) -> bool {
    summary.updated_by_run_ref.is_some()
        || !summary.artifact_refs.is_empty()
        || !summary.observation_refs.is_empty()
        || summary.coverage_items.iter().any(|item| {
            !item.supporting_run_refs.is_empty()
                || !item.observation_refs.is_empty()
                || !item.supporting_artifact_refs.is_empty()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use volicord_types::values::EvidenceStatus;

    fn empty_summary() -> EvidenceSummary {
        EvidenceSummary {
            evidence_state: Some(EvidenceDisplayState::Prepared),
            status: EvidenceStatus::Unknown,
            coverage_items: Vec::new(),
            artifact_refs: Vec::new(),
            observation_refs: Vec::new(),
            updated_by_run_ref: None,
        }
    }

    #[test]
    fn display_projection_uses_only_typed_evidence_and_close_basis_facts() {
        let projected = evidence_summary_for_display(empty_summary(), None);
        assert_eq!(projected.evidence_state, None);
    }
}
