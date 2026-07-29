use volicord_store::core_pipeline::CoreProjectStore;
use volicord_types::ids::TaskId;
use volicord_types::schema::{GuaranteeDisplay, WriteTicketStateSummary};
use volicord_types::values::UtcTimestamp;

use crate::pipeline::CoreResult;

use super::current_validity::{
    evaluate_current_write_ticket, evaluate_terminal_write_ticket,
    requires_sensitive_approval_facts, EvaluatedWriteTicket,
};
use super::read_model::{
    load_sensitive_approval_facts, load_write_ticket_candidates, load_write_ticket_control_facts,
    load_write_ticket_evidence_facts, WriteTicketCurrentFacts,
};
use super::selection::{select_write_ticket, WriteTicketSelection};
use super::summary::{project_write_ticket_summary, WriteTicketSummaryInput};

pub(crate) fn load_evaluated_write_tickets(
    store: &CoreProjectStore,
    task_id: &TaskId,
    observed_at: &UtcTimestamp,
) -> CoreResult<Vec<EvaluatedWriteTicket>> {
    let candidates = load_write_ticket_candidates(store, task_id)?;
    let needs_current_facts = candidates
        .iter()
        .any(|candidate| evaluate_terminal_write_ticket(candidate.clone(), observed_at).is_none());
    let current = if needs_current_facts {
        let (task, workflow) = load_write_ticket_control_facts(store, task_id)?;
        let needs_approval_facts = candidates.iter().any(|candidate| {
            requires_sensitive_approval_facts(candidate, &task, &workflow, observed_at)
        });
        let sensitive_approvals = if needs_approval_facts {
            load_sensitive_approval_facts(store, task_id, observed_at)?
        } else {
            Vec::new()
        };
        Some(WriteTicketCurrentFacts {
            task,
            workflow,
            sensitive_approvals,
            observed_at: observed_at.clone(),
        })
    } else {
        None
    };
    Ok(candidates
        .into_iter()
        .map(|candidate| {
            evaluate_terminal_write_ticket(candidate.clone(), observed_at).unwrap_or_else(|| {
                evaluate_current_write_ticket(
                    candidate,
                    current
                        .as_ref()
                        .expect("active ticket evaluation acquires current facts"),
                )
            })
        })
        .collect())
}

pub(crate) fn load_current_write_ticket_summary(
    store: &CoreProjectStore,
    task_id: &TaskId,
    state_version: u64,
    observed_at: &UtcTimestamp,
    guarantee_display: Option<GuaranteeDisplay>,
) -> CoreResult<Option<WriteTicketStateSummary>> {
    let selection = select_write_ticket(load_evaluated_write_tickets(store, task_id, observed_at)?);
    let WriteTicketSelection::Selected(evaluated) = selection else {
        return Ok(None);
    };
    let evidence = load_write_ticket_evidence_facts(
        store,
        &evaluated.ticket.validity_basis.task_id,
        evaluated.consumed_by_run_id.as_ref(),
        state_version,
    )?;
    Ok(Some(project_write_ticket_summary(
        WriteTicketSummaryInput {
            evaluated: &evaluated,
            state_version,
            evidence: &evidence,
            guarantee_display,
        },
    )))
}
