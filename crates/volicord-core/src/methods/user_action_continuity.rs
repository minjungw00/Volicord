use crate::methods::{
    decode_required_json, plan_project_continuity_record, user_action_service_plan_error,
    PlanError, PlannedProjectContinuityRecord, ProjectContinuityDraft,
    ProjectContinuityPlanContext,
};
use crate::pipeline::{CorePipelineError, CoreService};
use volicord_store::core_pipeline::{ChangeUnitRecord, CoreProjectStore, ProjectStateHeader};
use volicord_types::{
    ids::TaskId,
    schema::{
        StateRecordRef, ToolEnvelope, UserActionBasis, UserActionRequestBody,
        UserActionResolutionBody,
    },
    values::UtcTimestamp,
};
use volicord_user_action_service::{derive_user_action_continuity, UserActionContinuityInput};

#[allow(clippy::too_many_arguments)]
pub(crate) fn plan_user_action_continuity_records(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    current_change_unit: Option<&ChangeUnitRecord>,
    request_body: &UserActionRequestBody,
    basis: &UserActionBasis,
    resolution: &UserActionResolutionBody,
    resolution_ref: &StateRecordRef,
    now: &UtcTimestamp,
) -> Result<Vec<PlannedProjectContinuityRecord>, PlanError> {
    let applies_to_paths = current_change_unit
        .map(|record| {
            decode_required_json(
                "change_units",
                record.change_unit_id.clone(),
                "bounded_paths_json",
                Some(&record.bounded_paths_json),
            )
            .map_err(PlanError::Core)
        })
        .transpose()?
        .unwrap_or_default();
    let current_close_basis = store
        .task_revision_record(task_id)
        .map_err(CorePipelineError::from)?
        .and_then(|record| record.current_close_basis);
    let drafts = derive_user_action_continuity(UserActionContinuityInput {
        request_body,
        basis,
        resolution,
        resolution_ref,
        applies_to_paths,
        current_close_basis: current_close_basis.as_ref(),
    })
    .map_err(|error| user_action_service_plan_error(envelope, project_state, error))?;
    let continuity_context = ProjectContinuityPlanContext {
        service,
        store,
        project_id: &envelope.project_id,
        source_task_id: task_id,
        source_change_unit_id: basis.coordinates().change_unit_id.as_ref(),
        planned_state_version: project_state.state_version + 1,
        now,
    };
    drafts
        .into_iter()
        .map(|draft| {
            plan_project_continuity_record(
                continuity_context,
                ProjectContinuityDraft {
                    kind: draft.kind,
                    title: draft.title,
                    summary: draft.summary,
                    rationale: draft.rationale,
                    applies_to_paths: draft.applies_to_paths,
                    applies_to_refs: draft.applies_to_refs,
                    source_refs: draft.source_refs,
                    artifact_refs: draft.artifact_refs,
                    supersedes_refs: draft.supersedes_refs,
                    review_triggers: draft.review_triggers,
                    metadata: draft.metadata,
                },
            )
            .map_err(PlanError::Core)
        })
        .collect()
}
