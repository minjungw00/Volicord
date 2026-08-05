use volicord_store::core_pipeline::{
    AcceptanceCriterionRecord, EvidenceClaimRecord, EvidenceObservationRecord, RunRecord,
};
use volicord_types::ids::{BaselineRef, ChangeUnitId, ProjectId, TaskId};
use volicord_types::schema::{
    CurrentCloseBasis, EvidenceObservation, EvidenceTarget, StateRecordRef,
};
use volicord_types::values::{StateRecordKind, UtcTimestamp};

#[derive(Debug, Clone, Copy)]
pub(crate) struct EvidenceObservationBasis<'a> {
    pub(crate) project_id: &'a ProjectId,
    pub(crate) task_id: &'a TaskId,
    pub(crate) change_unit_id: &'a str,
    pub(crate) scope_revision: u64,
    pub(crate) baseline_ref: Option<&'a str>,
    pub(crate) target: &'a EvidenceTarget,
    pub(crate) now: &'a UtcTimestamp,
}

pub(crate) fn stored_observation_target_matches(
    record: &EvidenceObservationRecord,
    target: &EvidenceTarget,
) -> bool {
    match target {
        EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id,
        } => {
            record.acceptance_criterion_id.as_deref() == Some(acceptance_criterion_id.as_str())
                && record.evidence_claim_id.is_none()
        }
        EvidenceTarget::SupplementalClaim {
            evidence_claim_id, ..
        } => {
            record.evidence_claim_id.as_deref() == Some(evidence_claim_id.as_str())
                && record.acceptance_criterion_id.is_none()
        }
    }
}

pub(crate) fn acceptance_criterion_target_is_current(
    record: Option<&AcceptanceCriterionRecord>,
    task_id: &TaskId,
) -> bool {
    record.is_some_and(|record| {
        record.task_id == task_id.as_str()
            && record.status == volicord_store::core_pipeline::AcceptanceCriterionStatus::Active
    })
}

pub(crate) fn supplemental_claim_target_matches(
    record: Option<&EvidenceClaimRecord>,
    statement: &str,
) -> bool {
    record.is_none_or(|record| record.statement == statement)
}

pub(crate) fn projected_observation_matches_basis(
    observation: &EvidenceObservation,
    basis: &EvidenceObservationBasis<'_>,
) -> bool {
    observation.project_id == *basis.project_id
        && observation.task_id == *basis.task_id
        && observation
            .change_unit_id
            .as_ref()
            .map(ChangeUnitId::as_str)
            == Some(basis.change_unit_id)
        && observation.target == *basis.target
}

pub(crate) fn stored_observation_matches_basis(
    record: &EvidenceObservationRecord,
    source_run: Option<&RunRecord>,
    basis: &EvidenceObservationBasis<'_>,
) -> bool {
    record.project_id == basis.project_id.as_str()
        && record.task_id == basis.task_id.as_str()
        && record.change_unit_id.as_deref() == Some(basis.change_unit_id)
        && stored_observation_target_matches(record, basis.target)
        && source_run.is_some_and(|run| {
            run_record_matches_close_basis_context(
                run,
                basis.project_id,
                basis.task_id,
                basis.change_unit_id,
                basis.scope_revision,
                basis.baseline_ref,
            )
        })
}

pub(crate) fn projected_observation_matches_close_basis(
    observation: &EvidenceObservation,
    basis: &CurrentCloseBasis,
    target: &EvidenceTarget,
) -> bool {
    observation.change_unit_id.as_ref() == Some(&basis.change_unit_id)
        && observation.run_ref.as_ref().is_some_and(|run_ref| {
            basis
                .source_run_ref
                .as_ref()
                .is_some_and(|source| run_ref.record_id == source.record_id)
        })
        && observation.target == *target
}

pub(crate) fn stored_observation_matches_close_basis(
    record: &EvidenceObservationRecord,
    basis: &CurrentCloseBasis,
    target: &EvidenceTarget,
) -> bool {
    record.change_unit_id.as_deref() == Some(basis.change_unit_id.as_str())
        && basis
            .source_run_ref
            .as_ref()
            .is_some_and(|source| record.run_id.as_deref() == Some(source.record_id.as_str()))
        && stored_observation_target_matches(record, target)
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

pub(crate) fn close_basis_run_refs(basis: &CurrentCloseBasis) -> Vec<&StateRecordRef> {
    let mut refs = Vec::new();
    if let Some(source_run_ref) = basis.source_run_ref.as_ref() {
        if source_run_ref.record_kind == StateRecordKind::Run {
            refs.push(source_run_ref);
        }
    }
    refs.extend(
        basis
            .result_refs
            .iter()
            .filter(|record_ref| record_ref.record_kind == StateRecordKind::Run),
    );
    refs.extend(
        basis
            .residual_risks
            .iter()
            .flat_map(|risk| risk.source_refs.iter())
            .filter(|record_ref| record_ref.record_kind == StateRecordKind::Run),
    );
    refs
}

pub(crate) fn run_record_matches_close_basis_context(
    record: &RunRecord,
    project_id: &ProjectId,
    task_id: &TaskId,
    change_unit_id: &str,
    scope_revision: u64,
    baseline_ref: Option<&str>,
) -> bool {
    record.project_id == project_id.as_str()
        && record.task_id == task_id.as_str()
        && record.change_unit_id.as_deref() == Some(change_unit_id)
        && record.scope_revision == scope_revision
        && record.baseline_ref.as_ref().map(|value| value.as_str()) == baseline_ref
        && record.status == volicord_store::core_pipeline::RunStatus::Recorded
}

#[cfg(test)]
mod tests {
    use super::*;
    use volicord_store::core_pipeline::{AcceptanceCriterionStatus, RunStatus};
    use volicord_types::ids::{AcceptanceCriterionId, EvidenceObservationId, RecordId, RunId};
    use volicord_types::schema::{
        EvidenceProducerAnchor, EvidenceRelevanceAssessment, PersistedEvidenceObservationAuthority,
        RequiredNullable,
    };
    use volicord_types::values::{
        ActorSource, EvidenceAssuranceLevel, EvidenceProducerKind, EvidenceRelevanceStatus,
        EvidenceRequirement, EvidenceSourceKind,
    };

    fn target(id: &str) -> EvidenceTarget {
        EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id: AcceptanceCriterionId::new(id),
        }
    }

    fn observation(target: EvidenceTarget) -> EvidenceObservation {
        EvidenceObservation {
            observation_id: EvidenceObservationId::new("observation_target"),
            project_id: ProjectId::new("project_target"),
            task_id: TaskId::new("task_target"),
            change_unit_id: Some(ChangeUnitId::new("change_target")).into(),
            run_ref: Some(StateRecordRef {
                record_kind: StateRecordKind::Run,
                record_id: RecordId::new("run_target"),
                project_id: ProjectId::new("project_target"),
                task_id: Some(TaskId::new("task_target")).into(),
                produced_at_state_version: Some(7).into(),
            })
            .into(),
            target,
            source_kind: EvidenceSourceKind::ExternalTool,
            assurance_level: EvidenceAssuranceLevel::ExternalToolResult,
            producer_anchor: EvidenceProducerAnchor {
                producer_kind: EvidenceProducerKind::VerifiedToolInvocation,
                producer_ref: RequiredNullable::null(),
                output_artifact_refs: Vec::new(),
                verification_basis: RequiredNullable::null(),
            },
            relevance_assessment: EvidenceRelevanceAssessment {
                status: EvidenceRelevanceStatus::Supported,
                assessment_ref: RequiredNullable::null(),
                assessed_by_actor_source: RequiredNullable::null(),
            },
            observed_by_actor_source: Some(ActorSource::AgentConnection(
                volicord_types::ids::AgentConnectionId::new("connection_target"),
            ))
            .into(),
            tool_name: RequiredNullable::null(),
            tool_invocation_id: RequiredNullable::null(),
            tool_metadata: Default::default(),
            input_refs: Vec::new(),
            source_refs: Vec::new(),
            output_artifact_refs: Vec::new(),
            limitations: Vec::new(),
            observed_at: UtcTimestamp::parse("2026-07-26T00:00:00Z").unwrap(),
            recorded_at: UtcTimestamp::parse("2026-07-26T00:00:00Z").unwrap(),
        }
    }

    #[test]
    fn target_and_close_basis_mismatches_are_independent() {
        let expected_target = target("criterion_expected");
        let observation = observation(expected_target.clone());
        let basis = EvidenceObservationBasis {
            project_id: &ProjectId::new("project_target"),
            task_id: &TaskId::new("task_target"),
            change_unit_id: "change_target",
            scope_revision: 3,
            baseline_ref: Some("baseline_target"),
            target: &expected_target,
            now: &UtcTimestamp::parse("2026-07-26T00:00:00Z").unwrap(),
        };
        assert!(projected_observation_matches_basis(&observation, &basis));
        let stored_observation = EvidenceObservationRecord {
            project_id: "project_target".to_owned(),
            evidence_observation_id: "observation_target".to_owned(),
            task_id: "task_target".to_owned(),
            change_unit_id: Some("change_target".to_owned()),
            run_id: Some("run_target".to_owned()),
            acceptance_criterion_id: Some("criterion_expected".to_owned()),
            evidence_claim_id: None,
            source_kind: EvidenceSourceKind::ExternalTool,
            assurance_level: EvidenceAssuranceLevel::ExternalToolResult,
            observed_by_actor_source: None,
            tool_name: None,
            tool_invocation_id: None,
            tool_metadata: Default::default(),
            input_refs: Vec::new(),
            source_refs: Vec::new(),
            output_artifact_refs: Vec::new(),
            limitations: Vec::new(),
            observed_at: UtcTimestamp::parse("2026-07-26T00:00:00Z").unwrap(),
            recorded_at: UtcTimestamp::parse("2026-07-26T00:00:00Z").unwrap(),
            metadata: PersistedEvidenceObservationAuthority {
                recorded_by_run_id: RunId::new("run_target"),
                invocation_verification_basis: "verified".to_owned(),
                producer_anchor: EvidenceProducerAnchor {
                    producer_kind: EvidenceProducerKind::VerifiedToolInvocation,
                    producer_ref: RequiredNullable::null(),
                    output_artifact_refs: Vec::new(),
                    verification_basis: RequiredNullable::null(),
                },
                relevance_assessment: EvidenceRelevanceAssessment {
                    status: EvidenceRelevanceStatus::Supported,
                    assessment_ref: RequiredNullable::null(),
                    assessed_by_actor_source: RequiredNullable::null(),
                },
            },
        };
        let source_run = RunRecord {
            project_id: "project_target".to_owned(),
            run_id: "run_target".to_owned(),
            task_id: "task_target".to_owned(),
            change_unit_id: Some("change_target".to_owned()),
            scope_revision: 3,
            baseline_ref: Some(
                BaselineRef::parse("baseline_target").expect("canonical test BaselineRef"),
            ),
            status: RunStatus::Recorded,
        };
        assert_eq!(
            stored_observation_matches_basis(&stored_observation, Some(&source_run), &basis),
            projected_observation_matches_basis(&observation, &basis)
        );

        let mismatched_target = target("criterion_other");
        let target_mismatch = EvidenceObservationBasis {
            target: &mismatched_target,
            ..basis
        };
        assert_eq!(
            stored_observation_matches_basis(
                &stored_observation,
                Some(&source_run),
                &target_mismatch
            ),
            projected_observation_matches_basis(&observation, &target_mismatch)
        );
        assert!(!projected_observation_matches_basis(
            &observation,
            &target_mismatch
        ));

        let mut stale_close_basis = CurrentCloseBasis {
            close_basis_revision: 2,
            scope_revision: 3,
            task_id: TaskId::new("task_target"),
            change_unit_id: ChangeUnitId::new("change_target"),
            baseline_ref: Some(
                BaselineRef::parse("baseline_target").expect("canonical test BaselineRef"),
            )
            .into(),
            result_summary: "result".to_owned(),
            result_refs: Vec::new(),
            evidence_refs: Vec::new(),
            evidence_summary_ref: RequiredNullable::null(),
            residual_risks: Vec::new(),
            sensitive_categories: Vec::new(),
            sensitive_action_requirements: Vec::new(),
            recovery_constraints: Vec::new(),
            source_run_ref: RequiredNullable::some(observation.run_ref.as_ref().unwrap().clone()),
            shaping_checkpoint_ref: RequiredNullable::null(),
            shaping_decision_application_refs: Vec::new(),
            updated_at: UtcTimestamp::parse("2026-07-26T00:00:00Z").unwrap(),
        };
        assert!(projected_observation_matches_close_basis(
            &observation,
            &stale_close_basis,
            &expected_target
        ));
        stale_close_basis.change_unit_id = ChangeUnitId::new("change_stale");
        assert!(!projected_observation_matches_close_basis(
            &observation,
            &stale_close_basis,
            &expected_target
        ));
    }

    #[test]
    fn active_criterion_and_immutable_supplemental_targets_are_distinct() {
        let task_id = TaskId::new("task_target");
        let active = AcceptanceCriterionRecord {
            project_id: "project_target".to_owned(),
            acceptance_criterion_id: "criterion_target".to_owned(),
            task_id: task_id.as_str().to_owned(),
            statement: "criterion".to_owned(),
            evidence_requirement: EvidenceRequirement::Required,
            position: 1,
            status: AcceptanceCriterionStatus::Active,
        };
        assert!(acceptance_criterion_target_is_current(
            Some(&active),
            &task_id
        ));
        let mut retired = active;
        retired.status = AcceptanceCriterionStatus::Retired;
        assert!(!acceptance_criterion_target_is_current(
            Some(&retired),
            &task_id
        ));

        let claim = EvidenceClaimRecord {
            project_id: "project_target".to_owned(),
            evidence_claim_id: "claim_target".to_owned(),
            task_id: task_id.as_str().to_owned(),
            statement: "immutable claim".to_owned(),
        };
        assert!(supplemental_claim_target_matches(
            Some(&claim),
            "immutable claim"
        ));
        assert!(!supplemental_claim_target_matches(
            Some(&claim),
            "different claim"
        ));
    }
}
