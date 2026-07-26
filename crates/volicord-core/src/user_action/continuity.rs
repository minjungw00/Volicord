use crate::methods::{
    decision_rejected_response, decode_required_json, plan_project_continuity_record, PlanError,
    PlannedProjectContinuityRecord, ProjectContinuityDraft, ProjectContinuityPlanContext,
};
use crate::pipeline::{CorePipelineError, CoreService};
use crate::policy::continuity::{decision_title_prefix, judgment_continuity_kind};
use serde_json::json;
use std::collections::BTreeSet;
use volicord_store::core_pipeline::{ChangeUnitRecord, CoreProjectStore, ProjectStateHeader};
use volicord_types::ids::TaskId;
use volicord_types::schema::{
    StateRecordRef, ToolEnvelope, UserActionBasis, UserActionRequestBody, UserActionResolutionBody,
};
use volicord_types::values::{
    JudgmentResolutionOutcome, ProjectContinuityKind, UserActionOptionAction, UtcTimestamp,
};

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
    let (
        UserActionRequestBody::Choice(choice),
        UserActionBasis::Choice(choice_basis),
        UserActionResolutionBody::Choice {
            selected_option_id,
            machine_action,
            resolution_outcome,
            note: _,
            accepted_risk_ids,
        },
    ) = (request_body, basis, resolution)
    else {
        return Ok(Vec::new());
    };
    if *machine_action != UserActionOptionAction::Accept
        || *resolution_outcome != JudgmentResolutionOutcome::Accepted
    {
        return Ok(Vec::new());
    }
    let Some(continuity_kind) = judgment_continuity_kind(choice.judgment_kind, *resolution_outcome)
    else {
        return Ok(Vec::new());
    };
    let selected = choice
        .options
        .iter()
        .find(|option| option.option_id == *selected_option_id)
        .ok_or_else(|| {
            PlanError::Response(Box::new(decision_rejected_response(
                envelope,
                Some(project_state.state_version),
                "stored user-action resolution does not select a request option",
            )))
        })?;
    let source_change_unit_id = choice_basis.coordinates.change_unit_id.as_ref();
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
    let continuity_context = ProjectContinuityPlanContext {
        service,
        store,
        project_id: &envelope.project_id,
        source_task_id: task_id,
        source_change_unit_id,
        planned_state_version: project_state.state_version + 1,
        now,
    };
    match continuity_kind {
        ProjectContinuityKind::Decision => {
            let mut applies_to_refs = choice.affected_refs.clone();
            applies_to_refs.extend(choice.context.related_refs.clone());
            let mut source_refs = vec![resolution_ref.clone()];
            source_refs.extend(applies_to_refs.clone());
            let summary = format!(
                "Selected option: {}. {}",
                selected.label,
                choice.context.summary.trim()
            );
            let draft = ProjectContinuityDraft {
                kind: ProjectContinuityKind::Decision,
                title: format!(
                    "{}: {}",
                    decision_title_prefix(choice.judgment_kind),
                    selected.label.trim().to_owned()
                ),
                summary,
                rationale: None,
                applies_to_paths,
                applies_to_refs,
                source_refs,
                artifact_refs: choice.context.artifact_refs.clone(),
                supersedes_refs: Vec::new(),
                review_triggers: Vec::new(),
                metadata: json!({
                    "source": "resolve_user_action",
                    "action_kind": request_body.action_kind(),
                    "resolution_outcome": resolution_outcome,
                    "selected_option_id": selected_option_id
                }),
            };
            Ok(vec![plan_project_continuity_record(
                continuity_context,
                draft,
            )
            .map_err(PlanError::Core)?])
        }
        ProjectContinuityKind::AcceptedRisk => {
            if accepted_risk_ids.is_empty() {
                return Ok(Vec::new());
            }
            let close_basis = store
                .task_revision_record(task_id)
                .map_err(CorePipelineError::from)?
                .and_then(|record| record.current_close_basis)
                .ok_or_else(|| {
                    PlanError::Response(Box::new(decision_rejected_response(
                        envelope,
                        Some(project_state.state_version),
                        "accepted residual risks require the current close basis",
                    )))
                })?;
            let accepted = accepted_risk_ids.iter().collect::<BTreeSet<_>>();
            let risks = close_basis
                .residual_risks
                .iter()
                .filter(|risk| accepted.contains(&risk.risk_id))
                .collect::<Vec<_>>();
            if risks.len() != accepted.len() {
                return Err(PlanError::Response(Box::new(decision_rejected_response(
                    envelope,
                    Some(project_state.state_version),
                    "accepted residual-risk identities do not match the current close basis",
                ))));
            }
            let mut plans = Vec::with_capacity(risks.len());
            for risk in risks {
                let mut source_refs = vec![resolution_ref.clone()];
                source_refs.extend(risk.source_refs.clone());
                let mut applies_to_refs = close_basis.result_refs.clone();
                applies_to_refs.extend(risk.source_refs.clone());
                let draft = ProjectContinuityDraft {
                    kind: ProjectContinuityKind::AcceptedRisk,
                    title: format!("Accepted residual risk: {}", risk.summary.trim().to_owned()),
                    summary: risk.summary.clone(),
                    rationale: None,
                    applies_to_paths: applies_to_paths.clone(),
                    applies_to_refs,
                    source_refs,
                    artifact_refs: choice.context.artifact_refs.clone(),
                    supersedes_refs: Vec::new(),
                    review_triggers: Vec::new(),
                    metadata: json!({
                        "source": "resolve_user_action",
                        "action_kind": request_body.action_kind(),
                        "risk_id": risk.risk_id,
                        "close_basis_revision": close_basis.close_basis_revision
                    }),
                };
                plans.push(
                    plan_project_continuity_record(continuity_context, draft)
                        .map_err(PlanError::Core)?,
                );
            }
            Ok(plans)
        }
        _ => Ok(Vec::new()),
    }
}
