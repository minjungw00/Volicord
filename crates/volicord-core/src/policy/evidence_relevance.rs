use volicord_types::schema::EvidenceRelevanceAssessment;
use volicord_types::values::{EvidenceProducerKind, EvidenceRelevanceStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvidenceSupportClass {
    Supported,
    Unsupported,
    Unassessed,
}

pub(crate) fn classify_evidence_support(
    relevance: EvidenceRelevanceStatus,
) -> EvidenceSupportClass {
    match relevance {
        EvidenceRelevanceStatus::Supported => EvidenceSupportClass::Supported,
        EvidenceRelevanceStatus::Contradicted => EvidenceSupportClass::Unsupported,
        EvidenceRelevanceStatus::Unassessed => EvidenceSupportClass::Unassessed,
    }
}

pub(crate) fn relevance_supports_claim(relevance: &EvidenceRelevanceAssessment) -> bool {
    classify_evidence_support(relevance.status) == EvidenceSupportClass::Supported
}

pub(crate) fn capture_outcome_relevance(matches_expected_outcome: bool) -> EvidenceRelevanceStatus {
    if matches_expected_outcome {
        EvidenceRelevanceStatus::Unassessed
    } else {
        EvidenceRelevanceStatus::Contradicted
    }
}

pub(crate) fn capture_relevance_is_unsupported(
    producer_kind: EvidenceProducerKind,
    relevance: &EvidenceRelevanceAssessment,
) -> bool {
    matches!(
        producer_kind,
        EvidenceProducerKind::VerifiedToolInvocation
            | EvidenceProducerKind::VerifiedCommandExecution
    ) && !relevance_supports_claim(relevance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use volicord_types::schema::RequiredNullable;

    fn assessment(status: EvidenceRelevanceStatus) -> EvidenceRelevanceAssessment {
        EvidenceRelevanceAssessment {
            status,
            assessment_ref: RequiredNullable::null(),
            assessed_by_actor_source: RequiredNullable::null(),
        }
    }

    #[test]
    fn relevance_distinguishes_supported_unsupported_and_unassessed() {
        assert_eq!(
            classify_evidence_support(EvidenceRelevanceStatus::Supported),
            EvidenceSupportClass::Supported
        );
        assert_eq!(
            classify_evidence_support(EvidenceRelevanceStatus::Contradicted),
            EvidenceSupportClass::Unsupported
        );
        assert_eq!(
            classify_evidence_support(EvidenceRelevanceStatus::Unassessed),
            EvidenceSupportClass::Unassessed
        );
    }

    #[test]
    fn capture_provenance_does_not_imply_supported_relevance() {
        assert_eq!(
            capture_outcome_relevance(true),
            EvidenceRelevanceStatus::Unassessed
        );
        assert_eq!(
            capture_outcome_relevance(false),
            EvidenceRelevanceStatus::Contradicted
        );
        assert!(capture_relevance_is_unsupported(
            EvidenceProducerKind::VerifiedCommandExecution,
            &assessment(EvidenceRelevanceStatus::Unassessed),
        ));
        assert!(capture_relevance_is_unsupported(
            EvidenceProducerKind::VerifiedToolInvocation,
            &assessment(EvidenceRelevanceStatus::Contradicted),
        ));
        assert!(!capture_relevance_is_unsupported(
            EvidenceProducerKind::VerifiedToolInvocation,
            &assessment(EvidenceRelevanceStatus::Supported),
        ));
    }
}
