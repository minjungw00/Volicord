use std::collections::BTreeSet;
use volicord_types::ids::{
    AcceptanceCriterionId, ArtifactId, ChangeUnitId, DurableIdGenerator, DurableIdKind,
    EvidenceCaptureIntentId, EvidenceObservationId, EvidenceProducerId, ProjectContinuityRecordId,
    RiskId, RunId, ShapingCheckpointId, ShapingGapId, StagedArtifactHandleId, TaskId,
    UserActionRequestId, UserActionResolutionId, WriteTicketId, DURABLE_ID_RETRY_LIMIT,
};

use volicord_store::core_pipeline::CoreProjectStore;

use crate::pipeline::{CorePipelineError, CoreResult};

pub(crate) fn allocate_durable_id(
    generator: &dyn DurableIdGenerator,
    kind: DurableIdKind,
    mut exists: impl FnMut(&str) -> CoreResult<bool>,
) -> CoreResult<String> {
    for _ in 0..DURABLE_ID_RETRY_LIMIT {
        let candidate = generator.generate(kind)?;
        if !exists(&candidate)? {
            return Ok(candidate);
        }
    }

    Err(CorePipelineError::GeneratedIdCollision {
        kind,
        attempts: DURABLE_ID_RETRY_LIMIT,
    })
}

pub(crate) fn allocate_task_id(
    generator: &dyn DurableIdGenerator,
    store: &CoreProjectStore,
) -> CoreResult<TaskId> {
    allocate_durable_id(generator, DurableIdKind::Task, |candidate| {
        store
            .task_exists(&TaskId::new(candidate))
            .map_err(CorePipelineError::from)
    })
    .map(TaskId::new)
}

pub(crate) fn allocate_change_unit_id(
    generator: &dyn DurableIdGenerator,
    store: &CoreProjectStore,
) -> CoreResult<ChangeUnitId> {
    allocate_durable_id(generator, DurableIdKind::ChangeUnit, |candidate| {
        store
            .change_unit_id_exists(candidate)
            .map_err(CorePipelineError::from)
    })
    .map(ChangeUnitId::new)
}

pub(crate) fn allocate_shaping_checkpoint_id(
    generator: &dyn DurableIdGenerator,
    store: &CoreProjectStore,
) -> CoreResult<ShapingCheckpointId> {
    allocate_durable_id(generator, DurableIdKind::ShapingCheckpoint, |candidate| {
        store
            .shaping_checkpoint_id_exists(candidate)
            .map_err(CorePipelineError::from)
    })
    .map(ShapingCheckpointId::new)
}

pub(crate) fn allocate_shaping_gap_id(
    generator: &dyn DurableIdGenerator,
    store: &CoreProjectStore,
    reserved_ids: &BTreeSet<String>,
) -> CoreResult<ShapingGapId> {
    allocate_durable_id(generator, DurableIdKind::ShapingGap, |candidate| {
        if reserved_ids.contains(candidate) {
            return Ok(true);
        }
        store
            .shaping_gap_id_exists(candidate)
            .map_err(CorePipelineError::from)
    })
    .map(ShapingGapId::new)
}

pub(crate) fn allocate_user_action_resolution_id(
    generator: &dyn DurableIdGenerator,
    store: &CoreProjectStore,
) -> CoreResult<UserActionResolutionId> {
    allocate_durable_id(
        generator,
        DurableIdKind::UserActionResolution,
        |candidate| {
            store
                .user_action_resolution_record(candidate)
                .map(|record| record.is_some())
                .map_err(CorePipelineError::from)
        },
    )
    .map(UserActionResolutionId::new)
}

pub(crate) fn allocate_user_action_request_id(
    generator: &dyn DurableIdGenerator,
    store: &CoreProjectStore,
) -> CoreResult<UserActionRequestId> {
    allocate_durable_id(generator, DurableIdKind::UserActionRequest, |candidate| {
        store
            .user_action_request_id_exists(candidate)
            .map_err(CorePipelineError::from)
    })
    .map(UserActionRequestId::new)
}

pub(crate) fn allocate_write_ticket_id(
    generator: &dyn DurableIdGenerator,
    store: &CoreProjectStore,
) -> CoreResult<WriteTicketId> {
    allocate_durable_id(generator, DurableIdKind::WriteTicket, |candidate| {
        store
            .write_ticket_record(candidate)
            .map(|record| record.is_some())
            .map_err(CorePipelineError::from)
    })
    .map(WriteTicketId::new)
}

pub(crate) fn allocate_run_id(
    generator: &dyn DurableIdGenerator,
    store: &CoreProjectStore,
) -> CoreResult<RunId> {
    allocate_durable_id(generator, DurableIdKind::Run, |candidate| {
        store
            .run_id_exists(candidate)
            .map_err(CorePipelineError::from)
    })
    .map(RunId::new)
}

pub(crate) fn allocate_staged_artifact_handle_id(
    generator: &dyn DurableIdGenerator,
    store: &CoreProjectStore,
) -> CoreResult<StagedArtifactHandleId> {
    allocate_durable_id(generator, DurableIdKind::StagedArtifact, |candidate| {
        store
            .artifact_staging_record(candidate)
            .map(|record| record.is_some())
            .map_err(CorePipelineError::from)
    })
    .map(StagedArtifactHandleId::new)
}

pub(crate) fn allocate_artifact_id(
    generator: &dyn DurableIdGenerator,
    store: &CoreProjectStore,
) -> CoreResult<ArtifactId> {
    allocate_durable_id(generator, DurableIdKind::Artifact, |candidate| {
        store
            .artifact_record(candidate)
            .map(|record| record.is_some())
            .map_err(CorePipelineError::from)
    })
    .map(ArtifactId::new)
}

pub(crate) fn allocate_evidence_summary_id(
    generator: &dyn DurableIdGenerator,
    store: &CoreProjectStore,
) -> CoreResult<String> {
    allocate_durable_id(generator, DurableIdKind::Evidence, |candidate| {
        store
            .evidence_summary_exists(candidate)
            .map_err(CorePipelineError::from)
    })
}

pub(crate) fn allocate_acceptance_criterion_id(
    generator: &dyn DurableIdGenerator,
    store: &CoreProjectStore,
    reserved_ids: &BTreeSet<String>,
) -> CoreResult<AcceptanceCriterionId> {
    allocate_durable_id(generator, DurableIdKind::AcceptanceCriterion, |candidate| {
        if reserved_ids.contains(candidate) {
            return Ok(true);
        }
        store
            .acceptance_criterion_id_exists(candidate)
            .map_err(CorePipelineError::from)
    })
    .map(AcceptanceCriterionId::new)
}

pub(crate) fn allocate_evidence_observation_id(
    generator: &dyn DurableIdGenerator,
    store: &CoreProjectStore,
) -> CoreResult<EvidenceObservationId> {
    allocate_durable_id(generator, DurableIdKind::EvidenceObservation, |candidate| {
        store
            .evidence_observation_exists(candidate)
            .map_err(CorePipelineError::from)
    })
    .map(EvidenceObservationId::new)
}

pub(crate) fn allocate_evidence_capture_intent_id(
    generator: &dyn DurableIdGenerator,
    store: &CoreProjectStore,
) -> CoreResult<EvidenceCaptureIntentId> {
    allocate_durable_id(
        generator,
        DurableIdKind::EvidenceCaptureIntent,
        |candidate| {
            store
                .evidence_capture_intent_record(candidate)
                .map(|record| record.is_some())
                .map_err(CorePipelineError::from)
        },
    )
    .map(EvidenceCaptureIntentId::new)
}

pub(crate) fn allocate_evidence_producer_id(
    generator: &dyn DurableIdGenerator,
    store: &CoreProjectStore,
) -> CoreResult<EvidenceProducerId> {
    allocate_durable_id(generator, DurableIdKind::EvidenceProducer, |candidate| {
        store
            .evidence_producer_record(candidate)
            .map(|record| record.is_some())
            .map_err(CorePipelineError::from)
    })
    .map(EvidenceProducerId::new)
}

pub(crate) fn allocate_risk_id(
    generator: &dyn DurableIdGenerator,
    allocated_in_basis: &BTreeSet<String>,
) -> CoreResult<RiskId> {
    allocate_durable_id(generator, DurableIdKind::Risk, |candidate| {
        Ok(allocated_in_basis.contains(candidate))
    })
    .map(RiskId::new)
}

pub(crate) fn allocate_project_continuity_record_id(
    generator: &dyn DurableIdGenerator,
    store: &CoreProjectStore,
) -> CoreResult<ProjectContinuityRecordId> {
    allocate_durable_id(
        generator,
        DurableIdKind::ProjectContinuityRecord,
        |candidate| {
            store
                .project_continuity_record_exists(candidate)
                .map_err(CorePipelineError::from)
        },
    )
    .map(ProjectContinuityRecordId::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use volicord_types::ids::SequenceDurableIdGenerator;

    #[test]
    fn durable_id_allocation_retries_collisions_before_returning_a_fresh_identity() {
        let generator = SequenceDurableIdGenerator::new(["collision", "fresh"]);
        let mut observed = Vec::new();

        let allocated = allocate_durable_id(&generator, DurableIdKind::Task, |candidate| {
            observed.push(candidate.to_owned());
            Ok(candidate.ends_with("collision"))
        })
        .expect("the second identity is fresh");

        assert_eq!(observed, vec!["task_collision", "task_fresh"]);
        assert_eq!(allocated, "task_fresh");
    }

    #[test]
    fn durable_id_allocation_reports_the_owner_kind_after_retry_exhaustion() {
        let suffixes = (0..DURABLE_ID_RETRY_LIMIT).map(|index| format!("collision_{index}"));
        let generator = SequenceDurableIdGenerator::new(suffixes);

        let error = allocate_durable_id(&generator, DurableIdKind::WriteTicket, |_| Ok(true))
            .expect_err("every generated identity collides");

        assert!(matches!(
            error,
            CorePipelineError::GeneratedIdCollision {
                kind: DurableIdKind::WriteTicket,
                attempts: DURABLE_ID_RETRY_LIMIT,
            }
        ));
    }
}
