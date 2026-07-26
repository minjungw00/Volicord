use volicord_types::values::{EvidenceAssuranceLevel, EvidenceSourceKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvidenceProvenanceClass {
    Strong,
    CooperativeAgentReport,
    Weak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EvidenceProvenanceFacts {
    pub(crate) basis_matches: bool,
    pub(crate) source_kind: EvidenceSourceKind,
    pub(crate) assurance_level: EvidenceAssuranceLevel,
    pub(crate) artifact_binding_matches: bool,
    pub(crate) producer_binding_matches: bool,
}

pub(crate) fn evidence_assurance_matches_source(
    source_kind: EvidenceSourceKind,
    assurance_level: EvidenceAssuranceLevel,
) -> bool {
    match source_kind {
        EvidenceSourceKind::AgentReport => {
            assurance_level == EvidenceAssuranceLevel::CooperativeReport
        }
        EvidenceSourceKind::ExternalTool => {
            assurance_level == EvidenceAssuranceLevel::ExternalToolResult
        }
        EvidenceSourceKind::UserObservation => {
            assurance_level == EvidenceAssuranceLevel::UserObserved
        }
        EvidenceSourceKind::ReusedEvidence => matches!(
            assurance_level,
            EvidenceAssuranceLevel::ExternalToolResult | EvidenceAssuranceLevel::UserObserved
        ),
        EvidenceSourceKind::UnverifiedClaim => {
            assurance_level == EvidenceAssuranceLevel::Unverified
        }
    }
}

pub(crate) fn classify_evidence_provenance(
    facts: &EvidenceProvenanceFacts,
) -> EvidenceProvenanceClass {
    if !facts.basis_matches
        || !evidence_assurance_matches_source(facts.source_kind, facts.assurance_level)
    {
        return EvidenceProvenanceClass::Weak;
    }
    match (facts.source_kind, facts.assurance_level) {
        (EvidenceSourceKind::AgentReport, EvidenceAssuranceLevel::CooperativeReport) => {
            EvidenceProvenanceClass::CooperativeAgentReport
        }
        (EvidenceSourceKind::ExternalTool, EvidenceAssuranceLevel::ExternalToolResult)
        | (EvidenceSourceKind::UserObservation, EvidenceAssuranceLevel::UserObserved)
        | (
            EvidenceSourceKind::ReusedEvidence,
            EvidenceAssuranceLevel::ExternalToolResult | EvidenceAssuranceLevel::UserObserved,
        ) if facts.artifact_binding_matches && facts.producer_binding_matches => {
            EvidenceProvenanceClass::Strong
        }
        _ => EvidenceProvenanceClass::Weak,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(
        source_kind: EvidenceSourceKind,
        assurance_level: EvidenceAssuranceLevel,
    ) -> EvidenceProvenanceFacts {
        EvidenceProvenanceFacts {
            basis_matches: true,
            source_kind,
            assurance_level,
            artifact_binding_matches: true,
            producer_binding_matches: true,
        }
    }

    #[test]
    fn provenance_matrix_distinguishes_strong_cooperative_and_weak() {
        let strong = facts(
            EvidenceSourceKind::ExternalTool,
            EvidenceAssuranceLevel::ExternalToolResult,
        );
        assert_eq!(
            classify_evidence_provenance(&strong),
            EvidenceProvenanceClass::Strong
        );

        let cooperative = facts(
            EvidenceSourceKind::AgentReport,
            EvidenceAssuranceLevel::CooperativeReport,
        );
        assert_eq!(
            classify_evidence_provenance(&cooperative),
            EvidenceProvenanceClass::CooperativeAgentReport
        );

        for weak in [
            EvidenceProvenanceFacts {
                basis_matches: false,
                ..strong
            },
            EvidenceProvenanceFacts {
                artifact_binding_matches: false,
                ..strong
            },
            EvidenceProvenanceFacts {
                producer_binding_matches: false,
                ..strong
            },
            EvidenceProvenanceFacts {
                assurance_level: EvidenceAssuranceLevel::CooperativeReport,
                ..strong
            },
            facts(
                EvidenceSourceKind::UnverifiedClaim,
                EvidenceAssuranceLevel::Unverified,
            ),
        ] {
            assert_eq!(
                classify_evidence_provenance(&weak),
                EvidenceProvenanceClass::Weak
            );
        }
    }

    #[test]
    fn equivalent_stored_and_projected_facts_classify_identically() {
        let cases = [
            (
                EvidenceSourceKind::UserObservation,
                EvidenceAssuranceLevel::UserObserved,
                true,
                true,
                true,
                EvidenceProvenanceClass::Strong,
            ),
            (
                EvidenceSourceKind::AgentReport,
                EvidenceAssuranceLevel::CooperativeReport,
                true,
                false,
                false,
                EvidenceProvenanceClass::CooperativeAgentReport,
            ),
            (
                EvidenceSourceKind::ExternalTool,
                EvidenceAssuranceLevel::ExternalToolResult,
                false,
                true,
                true,
                EvidenceProvenanceClass::Weak,
            ),
        ];
        for (
            source_kind,
            assurance_level,
            basis_matches,
            artifact_binding_matches,
            producer_binding_matches,
            expected,
        ) in cases
        {
            let stored_facts = EvidenceProvenanceFacts {
                basis_matches,
                source_kind,
                assurance_level,
                artifact_binding_matches,
                producer_binding_matches,
            };
            let projected_facts = EvidenceProvenanceFacts {
                basis_matches,
                source_kind,
                assurance_level,
                artifact_binding_matches,
                producer_binding_matches,
            };
            assert_eq!(classify_evidence_provenance(&stored_facts), expected);
            assert_eq!(
                classify_evidence_provenance(&projected_facts),
                classify_evidence_provenance(&stored_facts)
            );
        }
    }
}
