use crate::close_readiness::{
    facts_from_projection, facts_with_pending_authorities,
    facts_with_projected_acceptance_criteria, facts_with_record_run_projection,
    plan_projected_close_readiness,
};
use crate::enforcement_facts::project_enforcement_profile;
use crate::guarantee_projection::guarantee_display;
use crate::pipeline::VerifiedInvocationContext;
use crate::policy::workflow::project_workflow_policy;
use crate::recording::RecordingError;
use crate::state_summary::{project_state_header, state_summary, StateSummaryInput};
use crate::write_ticket::current_validity::project_stored_write_ticket_consumption;
use crate::write_ticket::read_model::WriteTicketEvidenceFacts;
use crate::write_ticket::service::load_current_write_ticket_summary;
use crate::write_ticket::summary::{
    project_stored_write_ticket_summary, StoredWriteTicketSummaryInput,
};
use volicord_store::core_pipeline::{CoreProjectStore, ProjectStateHeader};
use volicord_types::schema::StateSummary;

use super::model::RecordRunPlannedMutations;

/// Acquires the Store-aware facts needed for the post-Run state projection.
///
/// This owner returns a typed state fact and never constructs a method result.
pub(super) fn acquire_record_run_state(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: &VerifiedInvocationContext,
    planned: &RecordRunPlannedMutations,
) -> Result<StateSummary, RecordingError> {
    let enforcement_profile = project_enforcement_profile(store)?;
    let guarantee_display = guarantee_display(
        &enforcement_profile,
        verified_invocation,
        planned.planned_state_version,
    );
    let project_policy = project_workflow_policy(store)
        .map_err(crate::pipeline::CorePipelineError::from)?
        .summary;
    let shaping_checkpoint = store.current_shaping_checkpoint(&planned.request.task_id)?;
    let shaping_authority = crate::workflow_projection::task_wide_shaping_authority(
        store,
        &planned.request.project_id,
        planned.planned_state_version,
        &planned.projected_task,
        Some(&planned.change_unit),
        shaping_checkpoint.as_ref(),
        &planned.plan_now,
    )?;
    let write_ticket_summary = if let Some(ticket) = &planned.write_ticket_scope {
        let evaluated =
            project_stored_write_ticket_consumption(ticket.reusable(), planned.run_id.clone());
        let evidence = WriteTicketEvidenceFacts {
            observation_refs: planned.observation_refs.clone(),
        };
        Some(project_stored_write_ticket_summary(
            StoredWriteTicketSummaryInput {
                evaluated: &evaluated,
                state_version: planned.planned_state_version,
                evidence: &evidence,
                guarantee_display: Some(guarantee_display.clone()),
            },
        ))
    } else {
        load_current_write_ticket_summary(
            store,
            &planned.request.task_id,
            planned.planned_state_version,
            &planned.plan_now,
            Some(guarantee_display.clone()),
        )?
    };
    let projected_project_state = project_state_header(
        project_state,
        planned.planned_state_version,
        project_state
            .active_task_id
            .clone()
            .or_else(|| Some(planned.request.task_id.as_str().to_owned())),
    );
    let close_plan = plan_projected_close_readiness(
        store,
        &projected_project_state,
        &planned.request.project_id,
        &planned.request.task_id,
        facts_with_pending_authorities(
            facts_with_projected_acceptance_criteria(
                facts_with_record_run_projection(
                    facts_from_projection(
                        planned.projected_task.clone(),
                        Some(planned.change_unit.clone()),
                        planned.current_close_basis.clone(),
                        planned.pending_user_action_refs.clone(),
                        planned.blocker_refs.clone(),
                        planned.projected_close_evidence_summary.clone(),
                        planned.plan_now.clone(),
                    ),
                    planned.run_ref.clone(),
                    planned.evidence_observations.clone(),
                    planned.registered_artifacts.clone(),
                ),
                &planned.acceptance_criteria,
            ),
            planned.pending_authorities.clone(),
        ),
    )
    .map_err(RecordingError::CloseReadiness)?;
    Ok(state_summary(StateSummaryInput {
        project_id: &planned.request.project_id,
        state_version: planned.planned_state_version,
        task: &planned.projected_task,
        current_change_unit: Some(&planned.change_unit),
        shaping_checkpoint: shaping_checkpoint.as_ref(),
        task_wide_shaping_authority: &shaping_authority,
        project_policy,
        acceptance_criteria: planned.acceptance_criteria.clone(),
        pending_user_action_refs: planned.pending_user_action_refs.clone(),
        blocker_refs: planned.blocker_refs.clone(),
        write_ticket_summary,
        evidence_summary: planned.projected_state_evidence_summary.clone(),
        evidence_gate: Some(close_plan.evidence_gate),
        close_state: Some(close_plan.close_state),
        close_blockers: close_plan.blockers,
        guarantee_display: Some(guarantee_display),
    })?)
}
