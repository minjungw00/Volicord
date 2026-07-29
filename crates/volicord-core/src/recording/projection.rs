use volicord_types::ids::{ChangeUnitId, TaskId};
use volicord_types::methods::RecordRunResultFields;
use volicord_types::schema::{JsonObject, RunSummary, StateSummary};

use crate::methods::MethodPlan;

use super::model::{RecordRunMutationPlan, RecordRunPlannedMutations};

pub(super) struct RecordRunProjection {
    task_id: TaskId,
    change_unit_id: ChangeUnitId,
    mutation_plan: RecordRunMutationPlan,
    event_payload: JsonObject,
    result_fields: RecordRunResultFields,
}

impl RecordRunProjection {
    pub(super) fn into_plan(self) -> MethodPlan<RecordRunResultFields> {
        MethodPlan {
            task_id: self.task_id,
            change_unit_id: Some(self.change_unit_id),
            storage_mutations: self.mutation_plan.into_storage_mutations(),
            event_payload: self.event_payload,
            result_fields: self.result_fields,
            next_actions: Vec::new(),
        }
    }
}

pub(super) fn project_record_run_result(
    planned: RecordRunPlannedMutations,
    state: StateSummary,
) -> RecordRunProjection {
    let RecordRunPlannedMutations {
        request,
        run_ref,
        normalized_observed_changes,
        registered_artifacts,
        evidence_observations,
        evidence_producers,
        recorded_evidence_summary,
        current_close_basis,
        blocker_refs,
        mutation_plan,
        event_payload,
        ..
    } = planned;
    let result_fields = RecordRunResultFields {
        run_summary: RunSummary {
            run_ref: run_ref.clone(),
            kind: request.kind,
            summary: request.summary.clone(),
            observed_changes: normalized_observed_changes.clone(),
            artifact_refs: registered_artifacts.clone(),
        },
        registered_artifacts: registered_artifacts.clone(),
        evidence_summary: recorded_evidence_summary.clone(),
        evidence_observations: evidence_observations.clone(),
        evidence_producers,
        current_close_basis: current_close_basis.clone(),
        blocker_refs,
        state,
    };
    RecordRunProjection {
        task_id: request.task_id,
        change_unit_id: request.change_unit_id,
        mutation_plan,
        event_payload,
        result_fields,
    }
}
