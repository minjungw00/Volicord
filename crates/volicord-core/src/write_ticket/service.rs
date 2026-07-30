use volicord_store::core_pipeline::CoreProjectStore;
use volicord_types::ids::TaskId;
use volicord_types::schema::{GuaranteeDisplay, WriteTicketStateSummary};
use volicord_types::values::UtcTimestamp;

use crate::pipeline::CorePipelineError;
use crate::pipeline::CoreResult;

use super::approval::{assess_write_ticket_approval, WriteTicketApprovalRequirement};
use super::current_validity::{
    evaluate_active_candidate, pre_evaluate_stored_write_ticket, StoredTicketPreEvaluation,
    StoredWriteTicketEvaluation, StoredWriteTicketStateError,
};
use super::read_model::{
    load_sensitive_approval_facts, load_write_ticket_candidates, load_write_ticket_control_facts,
    load_write_ticket_evidence_facts, WriteTicketCurrentFacts, WriteTicketCurrentTaskFacts,
};
use super::selection::{select_stored_write_ticket, StoredWriteTicketSelection};
use super::summary::{project_stored_write_ticket_summary, StoredWriteTicketSummaryInput};

pub fn load_evaluated_stored_write_tickets(
    store: &CoreProjectStore,
    task_id: &TaskId,
    observed_at: &UtcTimestamp,
) -> CoreResult<Vec<StoredWriteTicketEvaluation>> {
    let candidates = load_write_ticket_candidates(store, task_id)?;
    let mut evaluated = Vec::with_capacity(candidates.len());
    let mut active = Vec::new();
    for candidate in candidates {
        match pre_evaluate_stored_write_ticket(candidate, observed_at)
            .map_err(stored_state_error)?
        {
            StoredTicketPreEvaluation::Complete(terminal) => evaluated.push(terminal.into()),
            StoredTicketPreEvaluation::NeedsCurrentFacts(candidate) => active.push(candidate),
        }
    }
    if active.is_empty() {
        return Ok(evaluated);
    }

    let (task, workflow) = load_write_ticket_control_facts(store, task_id)?;
    let sensitive_approvals = load_sensitive_approval_facts(store, task_id, observed_at)?;
    let current = WriteTicketCurrentFacts {
        task: WriteTicketCurrentTaskFacts {
            pending_policy_reevaluation: task.pending_policy_reevaluation,
        },
        workflow,
    };
    for candidate in active {
        let ticket = candidate.semantic_facts();
        let requirement = WriteTicketApprovalRequirement::new(
            ticket.project_id(),
            task.scope_revision,
            task.effective_control_level,
            ticket.attempt_scope(),
            observed_at,
        );
        let assessment = assess_write_ticket_approval(
            &requirement,
            &sensitive_approvals,
            &ticket.validity_basis().approval_basis_refs,
        );
        evaluated.push(evaluate_active_candidate(candidate, &current, assessment).into());
    }
    Ok(evaluated)
}

pub(crate) fn load_current_write_ticket_summary(
    store: &CoreProjectStore,
    task_id: &TaskId,
    state_version: u64,
    observed_at: &UtcTimestamp,
    guarantee_display: Option<GuaranteeDisplay>,
) -> CoreResult<Option<WriteTicketStateSummary>> {
    let selection = select_stored_write_ticket(load_evaluated_stored_write_tickets(
        store,
        task_id,
        observed_at,
    )?);
    let StoredWriteTicketSelection::Selected(evaluated) = selection else {
        return Ok(None);
    };
    let ticket = evaluated.semantic_facts();
    let evidence = load_write_ticket_evidence_facts(
        store,
        &ticket.validity_basis().task_id,
        evaluated.consumed_by_run_id(),
        state_version,
    )?;
    Ok(Some(project_stored_write_ticket_summary(
        StoredWriteTicketSummaryInput {
            evaluated: &evaluated,
            state_version,
            evidence: &evidence,
            guarantee_display,
        },
    )))
}

fn stored_state_error(error: StoredWriteTicketStateError) -> CorePipelineError {
    CorePipelineError::Invariant {
        detail: format!(
            "Store-validated Write Ticket could not enter the Core stored type-state family: {error:?}"
        ),
    }
}
