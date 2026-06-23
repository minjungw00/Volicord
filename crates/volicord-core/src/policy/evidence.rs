use std::collections::BTreeSet;

use volicord_types::{ArtifactRef, EvidenceCoverageItem, EvidenceCoverageState, EvidenceStatus};

pub(crate) fn evidence_status_for_items(items: &[EvidenceCoverageItem]) -> EvidenceStatus {
    if items
        .iter()
        .any(|item| item.coverage_state == EvidenceCoverageState::Blocked)
    {
        return EvidenceStatus::Blocked;
    }
    let required = items
        .iter()
        .filter(|item| item.required_for_close)
        .collect::<Vec<_>>();
    if required.is_empty() {
        return EvidenceStatus::Unknown;
    }
    if required.iter().all(|item| {
        matches!(
            item.coverage_state,
            EvidenceCoverageState::Supported | EvidenceCoverageState::NotApplicable
        )
    }) {
        EvidenceStatus::Sufficient
    } else {
        EvidenceStatus::Insufficient
    }
}

pub(crate) fn unique_artifact_refs(artifact_refs: Vec<ArtifactRef>) -> Vec<ArtifactRef> {
    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();
    for artifact_ref in artifact_refs {
        if seen.insert(artifact_ref.artifact_id.as_str().to_owned()) {
            unique.push(artifact_ref);
        }
    }
    unique
}
