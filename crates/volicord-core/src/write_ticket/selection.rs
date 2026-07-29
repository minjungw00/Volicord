use std::cmp::Ordering;

use volicord_types::values::WriteTicketStatus;

use super::current_validity::EvaluatedWriteTicket;
use super::semantic::WriteTicketEvaluationIdentity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WriteTicketSelection {
    None,
    Selected(Box<EvaluatedWriteTicket>),
}

pub(crate) fn select_write_ticket(candidates: Vec<EvaluatedWriteTicket>) -> WriteTicketSelection {
    candidates
        .into_iter()
        .min_by(compare_candidates)
        .map(Box::new)
        .map(WriteTicketSelection::Selected)
        .unwrap_or(WriteTicketSelection::None)
}

fn compare_candidates(left: &EvaluatedWriteTicket, right: &EvaluatedWriteTicket) -> Ordering {
    status_priority(left.effective_status)
        .cmp(&status_priority(right.effective_status))
        .then_with(|| {
            right
                .ticket
                .basis_state_version
                .cmp(&left.ticket.basis_state_version)
        })
        .then_with(|| stored_ticket_id(left).cmp(stored_ticket_id(right)))
}

fn stored_ticket_id(ticket: &EvaluatedWriteTicket) -> &str {
    match &ticket.identity {
        WriteTicketEvaluationIdentity::Stored { write_ticket_id } => write_ticket_id.as_str(),
        WriteTicketEvaluationIdentity::Planned { .. } => {
            panic!("planned tickets are not persisted selection candidates")
        }
    }
}

const fn status_priority(status: WriteTicketStatus) -> u8 {
    match status {
        WriteTicketStatus::Active => 0,
        WriteTicketStatus::Invalidated => 1,
        WriteTicketStatus::Consumed => 2,
        WriteTicketStatus::Revoked => 3,
    }
}

#[cfg(test)]
mod tests {
    use volicord_types::values::WriteTicketStatus;

    use super::{select_write_ticket, WriteTicketSelection};
    use crate::write_ticket::semantic::test_support::evaluated_ticket;

    fn selected_id(selection: WriteTicketSelection) -> Option<String> {
        match selection {
            WriteTicketSelection::None => None,
            WriteTicketSelection::Selected(ticket) => {
                Some(ticket.stored_write_ticket_id()?.as_str().to_owned())
            }
        }
    }

    #[test]
    fn empty_candidates_select_nothing() {
        assert_eq!(select_write_ticket(Vec::new()), WriteTicketSelection::None);
    }

    #[test]
    fn active_status_has_priority_over_newer_terminal_tickets() {
        let selection = select_write_ticket(vec![
            evaluated_ticket("ticket-consumed", WriteTicketStatus::Consumed, 11),
            evaluated_ticket("ticket-active", WriteTicketStatus::Active, 4),
            evaluated_ticket("ticket-invalidated", WriteTicketStatus::Invalidated, 12),
            evaluated_ticket("ticket-revoked", WriteTicketStatus::Revoked, 13),
        ]);

        assert_eq!(selected_id(selection).as_deref(), Some("ticket-active"));
    }

    #[test]
    fn equal_status_prefers_newer_basis_then_stable_identity() {
        let newer = select_write_ticket(vec![
            evaluated_ticket("ticket-a", WriteTicketStatus::Invalidated, 7),
            evaluated_ticket("ticket-b", WriteTicketStatus::Invalidated, 8),
        ]);
        assert_eq!(selected_id(newer).as_deref(), Some("ticket-b"));

        let stable = select_write_ticket(vec![
            evaluated_ticket("ticket-b", WriteTicketStatus::Consumed, 8),
            evaluated_ticket("ticket-a", WriteTicketStatus::Consumed, 8),
        ]);
        assert_eq!(selected_id(stable).as_deref(), Some("ticket-a"));
    }
}
