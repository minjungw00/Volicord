mod artifact;
mod authority;
mod context;
mod evidence;
mod model;
mod plan;
mod state;

use crate::artifact::ArtifactPolicyError;
use crate::close_readiness::CloseReadinessError;
use crate::pipeline::CorePipelineError;
use crate::write_ticket::WriteTicketInvalidReason;
use volicord_store::core_pipeline::CoreStorageMutation;
use volicord_store::error::StoreError;
use volicord_types::ids::{BaselineRef, ChangeUnitId, ProjectId, RunId, TaskId, WriteTicketId};
use volicord_types::schema::{
    ArtifactInput, ArtifactRef, CloseAssessmentInput, CurrentCloseBasis, DryRunIntent,
    EvidenceCoverageUpdate, EvidenceObservation, EvidenceObservationInput, EvidenceProducer,
    EvidenceSummary, JsonObject, ObservedChanges, StateRecordRef, StateSummary,
};
use volicord_types::values::RunKind;
use volicord_user_action_service::UserActionServiceError;

pub(crate) use plan::plan_record_run;

pub(crate) struct RecordRunInput {
    project_id: ProjectId,
    dry_run: DryRunIntent,
    task_id: TaskId,
    change_unit_id: ChangeUnitId,
    kind: RunKind,
    run_id: Option<RunId>,
    baseline_ref: BaselineRef,
    write_ticket_id: Option<WriteTicketId>,
    performed_operation: Option<String>,
    summary: String,
    observed_changes: ObservedChanges,
    artifact_inputs: Vec<ArtifactInput>,
    evidence_updates: Vec<EvidenceCoverageUpdate>,
    evidence_observations: Vec<EvidenceObservationInput>,
    close_assessment: Option<CloseAssessmentInput>,
}

impl RecordRunInput {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        project_id: ProjectId,
        dry_run: DryRunIntent,
        task_id: TaskId,
        change_unit_id: ChangeUnitId,
        kind: RunKind,
        run_id: Option<RunId>,
        baseline_ref: BaselineRef,
        write_ticket_id: Option<WriteTicketId>,
        performed_operation: Option<String>,
        summary: String,
        observed_changes: ObservedChanges,
        artifact_inputs: Vec<ArtifactInput>,
        evidence_updates: Vec<EvidenceCoverageUpdate>,
        evidence_observations: Vec<EvidenceObservationInput>,
        close_assessment: Option<CloseAssessmentInput>,
    ) -> Self {
        Self {
            project_id,
            dry_run,
            task_id,
            change_unit_id,
            kind,
            run_id,
            baseline_ref,
            write_ticket_id,
            performed_operation,
            summary,
            observed_changes,
            artifact_inputs,
            evidence_updates,
            evidence_observations,
            close_assessment,
        }
    }

    pub(crate) fn project_id(&self) -> &ProjectId {
        &self.project_id
    }

    pub(crate) fn dry_run(&self) -> DryRunIntent {
        self.dry_run
    }

    pub(crate) fn task_id(&self) -> &TaskId {
        &self.task_id
    }

    pub(crate) fn change_unit_id(&self) -> &ChangeUnitId {
        &self.change_unit_id
    }

    pub(crate) fn baseline_ref(&self) -> &BaselineRef {
        &self.baseline_ref
    }

    pub(crate) fn close_assessment(&self) -> Option<&CloseAssessmentInput> {
        self.close_assessment.as_ref()
    }
}

pub(crate) struct RecordRunOperationPlan {
    effect: RecordRunEffect,
    result_facts: RecordRunResultFacts,
}

impl RecordRunOperationPlan {
    pub(crate) fn into_parts(self) -> (RecordRunEffect, RecordRunResultFacts) {
        (self.effect, self.result_facts)
    }
}

pub(crate) struct RecordRunEffect {
    task_id: TaskId,
    change_unit_id: ChangeUnitId,
    mutation_plan: model::RecordRunMutationPlan,
    event_payload: JsonObject,
}

impl RecordRunEffect {
    pub(crate) fn task_id(&self) -> &TaskId {
        &self.task_id
    }

    pub(crate) fn change_unit_id(&self) -> &ChangeUnitId {
        &self.change_unit_id
    }

    pub(crate) fn event_payload(&self) -> &JsonObject {
        &self.event_payload
    }

    pub(crate) fn into_storage_mutations(self) -> Vec<CoreStorageMutation> {
        self.mutation_plan.into_storage_mutations()
    }
}

pub(crate) struct RecordRunResultFacts {
    run_ref: StateRecordRef,
    kind: RunKind,
    summary: String,
    observed_changes: ObservedChanges,
    registered_artifacts: Vec<ArtifactRef>,
    evidence_summary: Option<EvidenceSummary>,
    evidence_observations: Vec<EvidenceObservation>,
    evidence_producers: Vec<EvidenceProducer>,
    current_close_basis: Option<CurrentCloseBasis>,
    blocker_refs: Vec<StateRecordRef>,
    state: StateSummary,
}

impl RecordRunResultFacts {
    pub(crate) fn run_ref(&self) -> &StateRecordRef {
        &self.run_ref
    }

    pub(crate) fn kind(&self) -> RunKind {
        self.kind
    }

    pub(crate) fn summary(&self) -> &str {
        &self.summary
    }

    pub(crate) fn observed_changes(&self) -> &ObservedChanges {
        &self.observed_changes
    }

    pub(crate) fn registered_artifacts(&self) -> &[ArtifactRef] {
        &self.registered_artifacts
    }

    pub(crate) fn evidence_summary(&self) -> Option<&EvidenceSummary> {
        self.evidence_summary.as_ref()
    }

    pub(crate) fn evidence_observations(&self) -> &[EvidenceObservation] {
        &self.evidence_observations
    }

    pub(crate) fn evidence_producers(&self) -> &[EvidenceProducer] {
        &self.evidence_producers
    }

    pub(crate) fn current_close_basis(&self) -> Option<&CurrentCloseBasis> {
        self.current_close_basis.as_ref()
    }

    pub(crate) fn blocker_refs(&self) -> &[StateRecordRef] {
        &self.blocker_refs
    }

    pub(crate) fn state(&self) -> &StateSummary {
        &self.state
    }
}

#[derive(Debug)]
pub(crate) enum RecordingError {
    Core(CorePipelineError),
    Store(StoreError),
    UserAction(UserActionServiceError),
    Artifact(ArtifactPolicyError),
    CloseReadiness(CloseReadinessError),
    Rejected(RecordingRejection),
}

#[derive(Debug)]
pub(crate) enum RecordingRejection {
    Validation {
        field: &'static str,
        message: &'static str,
    },
    NoActiveTask,
    RunKindIncompatible,
    TaskPhaseTransitionRequired,
    ChangeUnitRequired,
    ChangeUnitStale,
    BaselineStale,
    WorkspaceStale,
    ProductPathContainment {
        message: &'static str,
    },
    DecisionRejected {
        message: &'static str,
    },
    WriteTicketRequired,
    WriteTicketInvalid {
        reason: WriteTicketInvalidReason,
        message: &'static str,
    },
    EvidenceInsufficient {
        message: &'static str,
    },
    ArtifactInput {
        artifact_input_id: String,
        reason: &'static str,
        message: &'static str,
    },
    ArtifactMissing {
        message: &'static str,
    },
}

impl From<CorePipelineError> for RecordingError {
    fn from(error: CorePipelineError) -> Self {
        match error {
            CorePipelineError::Store(error) => Self::Store(error),
            error => Self::Core(error),
        }
    }
}

impl From<StoreError> for RecordingError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

impl From<serde_json::Error> for RecordingError {
    fn from(error: serde_json::Error) -> Self {
        Self::Core(CorePipelineError::from(error))
    }
}

impl From<UserActionServiceError> for RecordingError {
    fn from(error: UserActionServiceError) -> Self {
        Self::UserAction(error)
    }
}

pub(super) fn recording_validation_error<T>(
    field: &'static str,
    message: &'static str,
) -> Result<T, RecordingError> {
    Err(RecordingError::Rejected(RecordingRejection::Validation {
        field,
        message,
    }))
}

pub(super) fn recording_store_error(error: StoreError) -> RecordingError {
    RecordingError::Store(error)
}

pub(super) fn recording_user_action_error(error: UserActionServiceError) -> RecordingError {
    RecordingError::UserAction(error)
}
