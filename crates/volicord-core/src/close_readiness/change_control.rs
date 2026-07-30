use super::blockers::{close_blocker, open_write_ticket_close_blocker};
use super::facts::CloseReadinessFacts;
use super::guidance::{close_guidance, CloseGuidance};
use super::service::CloseReadinessRequest;
use super::CloseReadinessError;
use crate::pipeline::CorePipelineError;
use crate::pipeline::CoreResult;
use crate::policy::close_readiness::is_terminal_lifecycle;
use crate::policy::evidence::state_record_ref_identity_key;
use crate::policy::evidence_target::{
    close_basis_is_current, close_basis_run_refs, run_record_matches_close_basis_context,
};
use crate::record_refs::{change_unit_ref, state_ref};
use crate::task_state::StoredScope;
use crate::write_ticket::current_validity::StoredWriteTicketEvaluation;
use crate::write_ticket::service::load_evaluated_stored_write_tickets;
use std::collections::BTreeSet;
use volicord_store::core_pipeline::{CoreProjectStore, ProjectStateHeader};
use volicord_types::ids::BaselineRef;
use volicord_types::schema::{CloseReadinessBlocker, CurrentCloseBasis, StateRecordRef};
use volicord_types::values::{
    CloseIntent, CloseReadinessBlockerCategory, StateRecordKind, UtcTimestamp,
};

pub(super) fn terminal_blockers(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseReadinessRequest,
    context: &mut CloseReadinessFacts,
    now: &UtcTimestamp,
) -> Result<Vec<CloseReadinessBlocker>, CloseReadinessError> {
    let mut blockers = Vec::new();
    let task_ref = task_ref_for_close(request, project_state.state_version);
    if is_terminal_lifecycle(&context.task.lifecycle_phase)
        || project_state
            .active_task_id
            .as_deref()
            .is_some_and(|active_task_id| active_task_id != request.task_id.as_str())
    {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::Task,
            "task_not_closeable",
            "The addressed Task is not the current non-terminal Task.",
            vec![task_ref.clone()],
            vec![close_guidance(
                CloseGuidance::ReviewCurrentTask,
                vec![task_ref.clone()],
            )],
        ));
    }

    if request.intent == CloseIntent::Supersede {
        let superseding_ref = request.superseding_task_id.as_ref().map(|task_id| {
            state_ref(
                StateRecordKind::Task,
                task_id.as_str(),
                &request.project_id,
                Some(task_id),
                Some(project_state.state_version),
            )
        });
        let replacement = request
            .superseding_task_id
            .as_ref()
            .map(|task_id| store.task_record(task_id).map_err(CorePipelineError::from))
            .transpose()?
            .flatten();
        if replacement
            .as_ref()
            .map(|task| is_terminal_lifecycle(&task.lifecycle_phase))
            .unwrap_or(true)
        {
            blockers.push(close_blocker(
                CloseReadinessBlockerCategory::Task,
                "task_not_closeable",
                "superseding_task_id must identify a non-terminal Task in this project.",
                superseding_ref.into_iter().collect(),
                Vec::new(),
            ));
        }
    }

    if recovery_required(context)? {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::Recovery,
            "recovery_required",
            "A recovery constraint or active blocker must be resolved before this terminal transition.",
            context.blocker_refs.clone(),
            vec![close_guidance(
                CloseGuidance::ResolveRecoveryBlockers,
                context.blocker_refs.clone(),
            )],
        ));
    }

    if matches!(request.intent, CloseIntent::Check | CloseIntent::Complete) {
        blockers.extend(unresolved_write_ticket_blockers(
            store,
            project_state,
            request,
            context,
            now,
        )?);
    }

    Ok(blockers)
}

pub(super) fn completion_scope_blockers(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseReadinessRequest,
    context: &CloseReadinessFacts,
) -> Result<Vec<CloseReadinessBlocker>, CloseReadinessError> {
    let mut blockers = Vec::new();
    let task_ref = task_ref_for_close(request, project_state.state_version);
    if context
        .current_change_unit
        .as_ref()
        .map(|record| {
            record.status != volicord_store::core_pipeline::ChangeUnitStatus::Active
                || !record.is_current
        })
        .unwrap_or(true)
    {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::Scope,
            "missing_active_change_unit",
            "Completion requires a current active Change Unit.",
            vec![task_ref.clone()],
            vec![close_guidance(
                CloseGuidance::RestoreActiveChangeUnit,
                vec![task_ref.clone()],
            )],
        ));
    }

    if let Some(blocker) = current_close_basis_blocker(store, request, project_state, context)? {
        blockers.push(blocker);
    }

    Ok(blockers)
}

pub(super) fn completion_basis_blockers(
    project_state: &ProjectStateHeader,
    request: &CloseReadinessRequest,
    context: &CloseReadinessFacts,
) -> Result<Vec<CloseReadinessBlocker>, CloseReadinessError> {
    let mut blockers = Vec::new();
    let task_ref = task_ref_for_close(request, project_state.state_version);
    let change_unit_ref = context.current_change_unit.as_ref().map(|record| {
        change_unit_ref(
            &request.project_id,
            &request.task_id,
            record,
            project_state.state_version,
        )
    });

    if baseline_stale_for_close(context)? {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::Baseline,
            "baseline_stale",
            "The current close basis is stale against the current baseline.",
            change_unit_ref.clone().into_iter().collect(),
            vec![close_guidance(
                CloseGuidance::RefreshCurrentBasis,
                vec![task_ref.clone()],
            )],
        ));
    }

    if context
        .current_close_basis
        .as_ref()
        .is_some_and(|basis| !basis.recovery_constraints.is_empty())
    {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::Recovery,
            "recovery_required",
            "The current close basis records recovery constraints that must be resolved.",
            vec![task_ref.clone()],
            vec![close_guidance(
                CloseGuidance::ResolveRecoveryConstraints,
                vec![task_ref],
            )],
        ));
    }

    Ok(blockers)
}

pub(super) fn unrecorded_change_blockers(
    project_state: &ProjectStateHeader,
    request: &CloseReadinessRequest,
    context: &CloseReadinessFacts,
) -> Vec<CloseReadinessBlocker> {
    if context.unresolved_unrecorded_changes.is_empty() {
        return Vec::new();
    }

    let task_ref = task_ref_for_close(request, project_state.state_version);
    vec![close_blocker(
        CloseReadinessBlockerCategory::ConnectionCapability,
        "unresolved_unrecorded_changes",
        "Observed Product Repository changes still need reconciliation.",
        vec![task_ref.clone()],
        vec![close_guidance(
            CloseGuidance::ReconcileChanges,
            vec![task_ref],
        )],
    )]
}

fn unresolved_write_ticket_blockers(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseReadinessRequest,
    context: &mut CloseReadinessFacts,
    now: &UtcTimestamp,
) -> Result<Vec<CloseReadinessBlocker>, CloseReadinessError> {
    let task_ref = task_ref_for_close(request, project_state.state_version);
    if context.write_tickets.is_none() {
        let records = load_evaluated_stored_write_tickets(store, &request.task_id, now)
            .map_err(CloseReadinessError::Core)?;
        context.write_tickets = Some(records);
    }
    let Some(write_tickets) = context.write_tickets.as_ref() else {
        return Err(CloseReadinessError::Core(CorePipelineError::Invariant {
            detail: "close-readiness Write Ticket facts were not acquired".to_owned(),
        }));
    };
    Ok(open_write_ticket_blockers_from_evaluated(
        task_ref,
        project_state.state_version,
        write_tickets,
    ))
}

fn open_write_ticket_blockers_from_evaluated(
    task_ref: StateRecordRef,
    state_version: u64,
    write_tickets: &[StoredWriteTicketEvaluation],
) -> Vec<CloseReadinessBlocker> {
    let mut blockers = Vec::new();
    for record in write_tickets {
        if let Some(reusable) = record.as_reusable() {
            let semantic = reusable.semantic_facts();
            blockers.push(open_write_ticket_close_blocker(
                task_ref.clone(),
                state_ref(
                    StateRecordKind::WriteTicket,
                    reusable.write_ticket_id().as_str(),
                    semantic.project_id(),
                    Some(&semantic.validity_basis().task_id),
                    Some(state_version),
                ),
            ));
        }
    }
    blockers
}

fn current_close_basis_blocker(
    store: &CoreProjectStore,
    request: &CloseReadinessRequest,
    project_state: &ProjectStateHeader,
    context: &CloseReadinessFacts,
) -> Result<Option<CloseReadinessBlocker>, CloseReadinessError> {
    let task_ref = task_ref_for_close(request, project_state.state_version);
    let Some(basis) = context.current_close_basis.as_ref() else {
        return Ok(Some(close_blocker(
            CloseReadinessBlockerCategory::Task,
            "missing_current_close_basis",
            "Completion requires a current close basis recorded by volicord.record_run.",
            vec![task_ref.clone()],
            vec![close_guidance(
                CloseGuidance::RecordCurrentCloseBasis,
                vec![task_ref],
            )],
        )));
    };
    let current_change_unit_id = context
        .current_change_unit
        .as_ref()
        .map(|record| record.change_unit_id.as_str());
    let current_baseline = StoredScope::from_task(&context.task)?.baseline_ref;
    if !close_basis_is_current(
        basis,
        &request.task_id,
        current_change_unit_id,
        context.task.scope_revision,
        context.task.close_basis_revision,
        current_baseline.as_deref(),
    ) {
        Ok(Some(close_blocker(
            CloseReadinessBlockerCategory::Scope,
            "stale_current_close_basis",
            "The current close basis is stale against current Task scope.",
            vec![task_ref.clone()],
            vec![close_guidance(
                CloseGuidance::RecordFreshScopeBasis,
                vec![task_ref],
            )],
        )))
    } else if let Some(blocker) = incompatible_close_basis_run_refs_blocker(
        store,
        request,
        project_state,
        context,
        basis,
        current_baseline.as_deref(),
    )? {
        Ok(Some(blocker))
    } else {
        Ok(None)
    }
}

fn incompatible_close_basis_run_refs_blocker(
    store: &CoreProjectStore,
    request: &CloseReadinessRequest,
    project_state: &ProjectStateHeader,
    context: &CloseReadinessFacts,
    basis: &CurrentCloseBasis,
    current_baseline: Option<&str>,
) -> Result<Option<CloseReadinessBlocker>, CloseReadinessError> {
    let Some(current_change_unit) = context.current_change_unit.as_ref() else {
        return Ok(None);
    };
    let current_change_unit_id = current_change_unit.change_unit_id.as_str();
    let mut seen = BTreeSet::new();
    let mut incompatible_refs = Vec::new();
    for record_ref in close_basis_run_refs(basis) {
        let record_id = record_ref.record_id.as_str();
        if !seen.insert(state_record_ref_identity_key(record_ref)) {
            continue;
        }
        if record_ref.project_id != request.project_id
            || record_ref.task_id.as_ref() != Some(&request.task_id)
        {
            incompatible_refs.push(record_ref.clone());
            continue;
        }
        if context.projected_run_refs.iter().any(|projected_ref| {
            state_record_ref_identity_key(projected_ref)
                == state_record_ref_identity_key(record_ref)
        }) {
            continue;
        }
        let record = store
            .run_record(record_id)
            .map_err(CorePipelineError::from)?;
        if record.as_ref().is_none_or(|record| {
            !run_record_matches_close_basis_context(
                record,
                &request.project_id,
                &request.task_id,
                current_change_unit_id,
                context.task.scope_revision,
                current_baseline,
            )
        }) {
            incompatible_refs.push(record_ref.clone());
        }
    }

    if incompatible_refs.is_empty() {
        Ok(None)
    } else {
        let task_ref = task_ref_for_close(request, project_state.state_version);
        Ok(Some(close_blocker(
            CloseReadinessBlockerCategory::Scope,
            "stale_current_close_basis",
            "The current close basis contains Run refs that are not current for the Task scope.",
            incompatible_refs,
            vec![close_guidance(
                CloseGuidance::RecordFreshRunBasis,
                vec![task_ref],
            )],
        )))
    }
}

pub(super) fn task_ref_for_close(
    request: &CloseReadinessRequest,
    state_version: u64,
) -> StateRecordRef {
    state_ref(
        StateRecordKind::Task,
        request.task_id.as_str(),
        &request.project_id,
        Some(&request.task_id),
        Some(state_version),
    )
}

fn baseline_stale_for_close(context: &CloseReadinessFacts) -> CoreResult<bool> {
    let Some(basis) = context.current_close_basis.as_ref() else {
        return Ok(false);
    };
    let current_baseline = StoredScope::from_task(&context.task)?.baseline_ref;
    Ok(basis.baseline_ref.as_ref().map(BaselineRef::as_str) != current_baseline.as_deref())
}

fn recovery_required(context: &CloseReadinessFacts) -> CoreResult<bool> {
    if !context.blocker_refs.is_empty() {
        return Ok(true);
    }
    context
        .current_change_unit
        .as_ref()
        .map(|record| Ok(record.lifecycle.recovery_required))
        .transpose()
        .map(|value| value.unwrap_or(false))
}

#[cfg(test)]
#[path = "tests/change_control.rs"]
mod tests;
