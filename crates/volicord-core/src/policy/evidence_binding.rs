use volicord_types::schema::{
    ArtifactRef, EvidenceObservation, EvidenceProducerAnchor, StateRecordRef,
};
use volicord_types::values::{
    ActorSource, ArtifactAvailability, ArtifactIntegrityStatus, EvidenceProducerKind,
    EvidenceRelevanceStatus, StateRecordKind,
};

use super::evidence_target::EvidenceObservationBasis;

pub(crate) fn exact_artifact_ref_sets_match(
    historical: &[ArtifactRef],
    current: &[ArtifactRef],
) -> bool {
    if historical.is_empty() || historical.len() != current.len() {
        return false;
    }
    let mut historical = historical.iter().collect::<Vec<_>>();
    let mut current = current.iter().collect::<Vec<_>>();
    historical.sort_by(|left, right| left.artifact_id.cmp(&right.artifact_id));
    current.sort_by(|left, right| left.artifact_id.cmp(&right.artifact_id));
    if historical
        .windows(2)
        .any(|refs| refs[0].artifact_id == refs[1].artifact_id)
        || current
            .windows(2)
            .any(|refs| refs[0].artifact_id == refs[1].artifact_id)
    {
        return false;
    }
    historical
        .into_iter()
        .zip(current)
        .all(|(historical, current)| exact_artifact_identity_matches(historical, current))
}

pub(crate) fn exact_artifact_identity_matches(
    historical: &ArtifactRef,
    current: &ArtifactRef,
) -> bool {
    if historical.integrity_status != ArtifactIntegrityStatus::Verified
        || current.integrity_status != ArtifactIntegrityStatus::Verified
        || historical.availability != ArtifactAvailability::Available
        || current.availability != ArtifactAvailability::Available
    {
        return false;
    }
    let mut normalized_current = current.clone();
    match (
        historical.created_by_run_ref.as_ref(),
        normalized_current.created_by_run_ref.as_mut(),
    ) {
        (Some(historical_run), Some(current_run)) => {
            current_run.produced_at_state_version = historical_run
                .produced_at_state_version
                .as_ref()
                .copied()
                .into();
        }
        (None, None) => {}
        _ => return false,
    }
    historical == &normalized_current
}

pub(crate) fn producer_output_binding_matches(
    producer_anchor: &EvidenceProducerAnchor,
    output_artifact_refs: &[ArtifactRef],
) -> bool {
    exact_artifact_ref_sets_match(&producer_anchor.output_artifact_refs, output_artifact_refs)
}

pub(crate) fn authority_ref_matches(
    authority_ref: Option<&StateRecordRef>,
    expected: &StateRecordRef,
) -> bool {
    authority_ref.is_some_and(|authority_ref| {
        authority_ref.record_kind == expected.record_kind
            && authority_ref.record_id == expected.record_id
            && authority_ref.project_id == expected.project_id
            && authority_ref.task_id == expected.task_id
    })
}

pub(crate) fn projected_capture_binding_matches(
    observation: &EvidenceObservation,
    basis: &EvidenceObservationBasis<'_>,
    verification_basis: Option<&str>,
) -> bool {
    let Some(producer_ref) = observation.producer_anchor.producer_ref.as_ref() else {
        return false;
    };
    let Some(intent_ref) = observation.relevance_assessment.assessment_ref.as_ref() else {
        return false;
    };
    let capture_refs = observation
        .input_refs
        .iter()
        .filter(|record_ref| record_ref.record_kind == StateRecordKind::EvidenceCaptureIntent)
        .collect::<Vec<_>>();
    producer_ref.record_kind == StateRecordKind::EvidenceProducer
        && producer_ref.project_id == *basis.project_id
        && producer_ref.task_id.as_ref() == Some(basis.task_id)
        && intent_ref.record_kind == StateRecordKind::EvidenceCaptureIntent
        && intent_ref.project_id == *basis.project_id
        && intent_ref.task_id.as_ref() == Some(basis.task_id)
        && capture_refs.as_slice() == [intent_ref]
        && matches!(
            observation.relevance_assessment.status,
            EvidenceRelevanceStatus::Unassessed | EvidenceRelevanceStatus::Contradicted
        )
        && observation
            .relevance_assessment
            .assessed_by_actor_source
            .is_none()
        && verification_basis.is_some()
        && observation.producer_anchor.verification_basis.as_deref() == verification_basis
        && observation
            .observed_by_actor_source
            .as_ref()
            .and_then(ActorSource::agent_connection_id)
            .is_some()
        && matches!(
            observation.producer_anchor.producer_kind,
            EvidenceProducerKind::VerifiedCommandExecution
                | EvidenceProducerKind::VerifiedToolInvocation
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use volicord_types::ids::{ArtifactId, ProjectId, StorageRef, TaskId};
    use volicord_types::schema::RequiredNullable;
    use volicord_types::values::RedactionState;

    fn artifact(id: &str) -> ArtifactRef {
        ArtifactRef {
            artifact_id: ArtifactId::new(id),
            project_id: ProjectId::new("project_binding"),
            task_id: TaskId::new("task_binding"),
            display_name: "artifact".to_owned(),
            content_type: Some("application/json".to_owned()).into(),
            sha256: Some("a".repeat(64)).into(),
            size_bytes: Some(4).into(),
            integrity_status: ArtifactIntegrityStatus::Verified,
            redaction_state: RedactionState::Redacted,
            availability: ArtifactAvailability::Available,
            created_by_run_ref: RequiredNullable::null(),
            created_by_actor_source: RequiredNullable::null(),
            storage_ref: Some(StorageRef::new("artifact-binding")).into(),
        }
    }

    #[test]
    fn artifact_binding_rejects_mismatch_and_duplicates() {
        let canonical = artifact("artifact_a");
        assert!(exact_artifact_ref_sets_match(
            std::slice::from_ref(&canonical),
            std::slice::from_ref(&canonical)
        ));

        let mut mismatched = canonical.clone();
        mismatched.sha256 = Some("b".repeat(64)).into();
        assert!(!exact_artifact_ref_sets_match(
            std::slice::from_ref(&canonical),
            std::slice::from_ref(&mismatched)
        ));
        assert!(!exact_artifact_ref_sets_match(
            &[canonical.clone(), canonical.clone()],
            &[canonical.clone(), canonical]
        ));
    }

    #[test]
    fn producer_binding_requires_exact_output_set() {
        let canonical = artifact("artifact_a");
        let anchor = EvidenceProducerAnchor {
            producer_kind: EvidenceProducerKind::VerifiedCommandExecution,
            producer_ref: RequiredNullable::null(),
            output_artifact_refs: vec![canonical.clone()],
            verification_basis: RequiredNullable::null(),
        };
        assert!(producer_output_binding_matches(
            &anchor,
            std::slice::from_ref(&canonical)
        ));
        assert!(!producer_output_binding_matches(
            &anchor,
            &[artifact("artifact_b")]
        ));
    }

    #[test]
    fn producer_authority_binding_rejects_identity_mismatch() {
        let expected = StateRecordRef {
            record_kind: StateRecordKind::EvidenceProducer,
            record_id: volicord_types::ids::RecordId::new("producer_binding"),
            project_id: ProjectId::new("project_binding"),
            task_id: Some(TaskId::new("task_binding")).into(),
            produced_at_state_version: Some(3).into(),
        };
        assert!(authority_ref_matches(Some(&expected), &expected));

        let mut mismatched = expected.clone();
        mismatched.record_id = volicord_types::ids::RecordId::new("producer_other");
        assert!(!authority_ref_matches(Some(&mismatched), &expected));

        mismatched = expected.clone();
        mismatched.project_id = ProjectId::new("project_other");
        assert!(!authority_ref_matches(Some(&mismatched), &expected));

        mismatched = expected.clone();
        mismatched.task_id = Some(TaskId::new("task_other")).into();
        assert!(!authority_ref_matches(Some(&mismatched), &expected));
    }
}
