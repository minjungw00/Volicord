use volicord_types::ids::RunId;
use volicord_types::values::{WriteTicketInvalidationReason, WriteTicketStatus};

use super::approval::WriteTicketApprovalAssessment;
use super::planning::PlannedWriteTicket;
use super::policy::write_ticket_is_idle_expired;
use super::read_model::WriteTicketCurrentFacts;
use super::semantic::{
    planned_write_ticket_semantic_facts, StoredWriteTicketFacts, WriteTicketEvaluationIdentity,
    WriteTicketSemanticFacts,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WriteTicketAuthorityState {
    NotApplicable,
    Current,
    WriteAuthorityChanged,
    PendingPolicyReevaluation,
}

/// Fully evaluated Write Ticket state consumed by Core policy and adapter projections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluatedWriteTicket {
    pub(crate) identity: WriteTicketEvaluationIdentity,
    pub(crate) ticket: WriteTicketSemanticFacts,
    pub(crate) effective_status: WriteTicketStatus,
    pub(crate) invalidation: Option<WriteTicketInvalidationReason>,
    pub(crate) authority: WriteTicketAuthorityState,
    pub(crate) consumed_by_run_id: Option<RunId>,
}

impl EvaluatedWriteTicket {
    pub fn stored_write_ticket_id(&self) -> Option<&volicord_types::ids::WriteTicketId> {
        match &self.identity {
            WriteTicketEvaluationIdentity::Stored { write_ticket_id } => Some(write_ticket_id),
            WriteTicketEvaluationIdentity::Planned { .. } => None,
        }
    }

    pub fn invalidation(&self) -> Option<WriteTicketInvalidationReason> {
        self.invalidation
    }
}

/// Evaluates lifecycle facts that do not require current Store-backed context.
///
/// `None` means the ticket remains active and needs current authority and
/// approval facts before it is fully evaluated.
pub(crate) fn evaluate_terminal_write_ticket(
    ticket: StoredWriteTicketFacts,
    observed_at: &volicord_types::values::UtcTimestamp,
) -> Option<EvaluatedWriteTicket> {
    if ticket.status != WriteTicketStatus::Active {
        let status = ticket.status;
        let invalidation_reason = ticket.invalidation_reason;
        return Some(evaluated_stored_ticket(
            ticket,
            status,
            invalidation_reason,
            WriteTicketAuthorityState::NotApplicable,
        ));
    }
    if write_ticket_is_idle_expired(ticket.ticket.idle_expires_at.as_ref(), observed_at) {
        return Some(evaluated_stored_ticket(
            ticket,
            WriteTicketStatus::Invalidated,
            Some(WriteTicketInvalidationReason::IdleTimeout),
            WriteTicketAuthorityState::NotApplicable,
        ));
    }
    None
}

pub(crate) fn evaluate_current_write_ticket(
    ticket: StoredWriteTicketFacts,
    current: &WriteTicketCurrentFacts,
    approval: WriteTicketApprovalAssessment,
) -> EvaluatedWriteTicket {
    let basis = &ticket.ticket.validity_basis;
    if basis.write_authority_fingerprint != current.workflow.write_authority_fingerprint {
        return evaluated_stored_ticket(
            ticket,
            WriteTicketStatus::Invalidated,
            Some(WriteTicketInvalidationReason::ExplicitRevoke),
            WriteTicketAuthorityState::WriteAuthorityChanged,
        );
    }
    if current.task.pending_policy_reevaluation {
        return evaluated_stored_ticket(
            ticket,
            WriteTicketStatus::Invalidated,
            Some(WriteTicketInvalidationReason::ExplicitRevoke),
            WriteTicketAuthorityState::PendingPolicyReevaluation,
        );
    }

    if matches!(approval, WriteTicketApprovalAssessment::Changed { .. }) {
        return evaluated_stored_ticket(
            ticket,
            WriteTicketStatus::Invalidated,
            Some(WriteTicketInvalidationReason::ApprovalBasisChanged),
            WriteTicketAuthorityState::Current,
        );
    }

    evaluated_stored_ticket(
        ticket,
        WriteTicketStatus::Active,
        None,
        WriteTicketAuthorityState::Current,
    )
}

pub(crate) fn evaluate_planned_write_ticket(plan: &PlannedWriteTicket) -> EvaluatedWriteTicket {
    let ticket = planned_write_ticket_semantic_facts(plan);
    EvaluatedWriteTicket {
        identity: WriteTicketEvaluationIdentity::Planned {
            write_ticket_id: plan.write_ticket_id().clone(),
        },
        ticket,
        effective_status: WriteTicketStatus::Active,
        invalidation: None,
        authority: WriteTicketAuthorityState::Current,
        consumed_by_run_id: None,
    }
}

pub(crate) fn evaluate_reused_write_ticket(ticket: StoredWriteTicketFacts) -> EvaluatedWriteTicket {
    debug_assert_eq!(ticket.status, WriteTicketStatus::Active);
    evaluated_stored_ticket(
        ticket,
        WriteTicketStatus::Active,
        None,
        WriteTicketAuthorityState::Current,
    )
}

pub(crate) fn evaluate_projected_write_ticket_consumption(
    ticket: StoredWriteTicketFacts,
    run_id: RunId,
) -> EvaluatedWriteTicket {
    let mut evaluated = evaluated_stored_ticket(
        ticket,
        WriteTicketStatus::Consumed,
        None,
        WriteTicketAuthorityState::NotApplicable,
    );
    evaluated.consumed_by_run_id = Some(run_id);
    evaluated
}

fn evaluated_stored_ticket(
    ticket: StoredWriteTicketFacts,
    effective_status: WriteTicketStatus,
    invalidation: Option<WriteTicketInvalidationReason>,
    authority: WriteTicketAuthorityState,
) -> EvaluatedWriteTicket {
    EvaluatedWriteTicket {
        identity: WriteTicketEvaluationIdentity::Stored {
            write_ticket_id: ticket.write_ticket_id,
        },
        ticket: ticket.ticket,
        effective_status,
        invalidation,
        authority,
        consumed_by_run_id: ticket.consumed_by_run_id,
    }
}

#[cfg(test)]
mod tests {
    use volicord_types::values::{
        TaskControlLevel, WriteTicketInvalidationReason, WriteTicketStatus,
    };

    use super::{
        evaluate_current_write_ticket, evaluate_terminal_write_ticket, WriteTicketAuthorityState,
    };
    use crate::write_ticket::approval::{ApprovalBasisChangeReason, WriteTicketApprovalAssessment};
    use crate::write_ticket::read_model::{
        WriteTicketCurrentFacts, WriteTicketTaskFacts, WriteTicketWorkflowFacts,
    };
    use crate::write_ticket::semantic::test_support::{stored_facts, timestamp};

    fn task_facts() -> WriteTicketTaskFacts {
        WriteTicketTaskFacts {
            scope_revision: 3,
            effective_control_level: TaskControlLevel::Tracked,
            pending_policy_reevaluation: false,
        }
    }

    fn workflow_facts() -> WriteTicketWorkflowFacts {
        WriteTicketWorkflowFacts {
            write_authority_fingerprint: format!("sha256:{}", "0".repeat(64)),
        }
    }

    fn current_facts() -> WriteTicketCurrentFacts {
        WriteTicketCurrentFacts {
            task: task_facts(),
            workflow: workflow_facts(),
            sensitive_approvals: Vec::new(),
            observed_at: timestamp("2026-07-29T00:05:00Z"),
        }
    }

    #[test]
    fn terminal_status_and_idle_expiry_do_not_require_current_facts() {
        let revoked = stored_facts("ticket-revoked", WriteTicketStatus::Revoked, 7);
        let evaluated = evaluate_terminal_write_ticket(revoked, &timestamp("2026-07-29T00:05:00Z"))
            .expect("terminal ticket should evaluate");
        assert_eq!(evaluated.effective_status, WriteTicketStatus::Revoked);
        assert_eq!(
            evaluated.authority,
            WriteTicketAuthorityState::NotApplicable
        );

        let active = stored_facts("ticket-expired", WriteTicketStatus::Active, 7);
        let evaluated = evaluate_terminal_write_ticket(active, &timestamp("2026-07-29T00:15:00Z"))
            .expect("expired active ticket should evaluate");
        assert_eq!(evaluated.effective_status, WriteTicketStatus::Invalidated);
        assert_eq!(
            evaluated.invalidation,
            Some(WriteTicketInvalidationReason::IdleTimeout)
        );
    }

    #[test]
    fn authority_change_and_pending_reevaluation_have_typed_results() {
        let mut changed = current_facts();
        changed.workflow.write_authority_fingerprint = format!("sha256:{}", "1".repeat(64));
        let evaluated = evaluate_current_write_ticket(
            stored_facts("ticket-changed", WriteTicketStatus::Active, 7),
            &changed,
            WriteTicketApprovalAssessment::NotRequired,
        );
        assert_eq!(evaluated.effective_status, WriteTicketStatus::Invalidated);
        assert_eq!(
            evaluated.authority,
            WriteTicketAuthorityState::WriteAuthorityChanged
        );
        assert_eq!(
            evaluated.invalidation,
            Some(WriteTicketInvalidationReason::ExplicitRevoke)
        );

        let mut pending = current_facts();
        pending.task.pending_policy_reevaluation = true;
        let evaluated = evaluate_current_write_ticket(
            stored_facts("ticket-pending", WriteTicketStatus::Active, 7),
            &pending,
            WriteTicketApprovalAssessment::NotRequired,
        );
        assert_eq!(
            evaluated.authority,
            WriteTicketAuthorityState::PendingPolicyReevaluation
        );
    }

    #[test]
    fn semantic_approval_assessment_drives_current_status() {
        let changed = evaluate_current_write_ticket(
            stored_facts("ticket-sensitive", WriteTicketStatus::Active, 7),
            &current_facts(),
            WriteTicketApprovalAssessment::Changed {
                reason: ApprovalBasisChangeReason::NoCurrentResolution,
            },
        );
        assert_eq!(
            changed.invalidation,
            Some(WriteTicketInvalidationReason::ApprovalBasisChanged)
        );
        let current = evaluate_current_write_ticket(
            stored_facts("ticket-current", WriteTicketStatus::Active, 7),
            &current_facts(),
            WriteTicketApprovalAssessment::NotRequired,
        );
        assert_eq!(current.effective_status, WriteTicketStatus::Active);
        assert_eq!(current.authority, WriteTicketAuthorityState::Current);
    }
}
