use std::collections::BTreeSet;

use volicord_types::{
    ArtifactRef, EvidenceCoverageItem, EvidenceCoverageState, EvidenceStatus, RecordId,
    StateRecordKind, StateRecordRef,
};

pub(crate) fn evidence_status_for_items(items: &[EvidenceCoverageItem]) -> EvidenceStatus {
    if items
        .iter()
        .any(|item| item.coverage_state == EvidenceCoverageState::Contradicted)
    {
        return EvidenceStatus::Blocked;
    }
    if items.is_empty() {
        return EvidenceStatus::Unknown;
    }
    if items.iter().all(|item| {
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

pub(crate) fn unique_state_record_refs(refs: Vec<StateRecordRef>) -> Vec<StateRecordRef> {
    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();
    for record_ref in refs {
        let key = state_record_ref_identity_key(&record_ref);
        if seen.insert(key) {
            unique.push(record_ref);
        }
    }
    unique
}

pub(crate) fn evidence_item_has_no_support(item: &EvidenceCoverageItem) -> bool {
    item.supporting_run_refs.is_empty()
        && item.observation_refs.is_empty()
        && item.supporting_artifact_refs.is_empty()
        && item.gap_refs.is_empty()
}

pub(crate) fn evidence_item_related_refs(item: &EvidenceCoverageItem) -> Vec<StateRecordRef> {
    let mut refs = Vec::new();
    refs.extend(item.observation_refs.clone());
    refs.extend(item.supporting_run_refs.clone());
    refs.extend(item.gap_refs.clone());
    refs.extend(item.supporting_artifact_refs.iter().map(|artifact_ref| {
        StateRecordRef {
            record_kind: StateRecordKind::Artifact,
            record_id: RecordId::new(artifact_ref.artifact_id.as_str()),
            project_id: artifact_ref.project_id.clone(),
            task_id: Some(artifact_ref.task_id.clone()).into(),
            produced_at_state_version: artifact_ref
                .created_by_run_ref
                .as_ref()
                .and_then(|record_ref| record_ref.produced_at_state_version.as_ref().copied())
                .into(),
        }
    }));
    refs
}

pub(crate) fn state_record_ref_identity_key(
    record_ref: &StateRecordRef,
) -> (String, String, String) {
    (
        record_ref.project_id.as_str().to_owned(),
        serde_json::to_string(&record_ref.record_kind)
            .expect("serializing a closed record-kind enum cannot fail"),
        record_ref.record_id.as_str().to_owned(),
    )
}
