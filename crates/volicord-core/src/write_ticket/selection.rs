use std::cmp::Ordering;

use volicord_types::values::WriteTicketStatus;

use super::current_validity::StoredWriteTicketEvaluation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StoredWriteTicketSelection {
    None,
    Selected(Box<StoredWriteTicketEvaluation>),
}

pub(crate) fn select_stored_write_ticket(
    candidates: Vec<StoredWriteTicketEvaluation>,
) -> StoredWriteTicketSelection {
    candidates
        .into_iter()
        .min_by(compare_candidates)
        .map(Box::new)
        .map(StoredWriteTicketSelection::Selected)
        .unwrap_or(StoredWriteTicketSelection::None)
}

fn compare_candidates(
    left: &StoredWriteTicketEvaluation,
    right: &StoredWriteTicketEvaluation,
) -> Ordering {
    status_priority(left.status())
        .cmp(&status_priority(right.status()))
        .then_with(|| {
            right
                .semantic_facts()
                .basis_state_version
                .cmp(&left.semantic_facts().basis_state_version)
        })
        .then_with(|| left.write_ticket_id().cmp(right.write_ticket_id()))
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

    use super::{select_stored_write_ticket, StoredWriteTicketSelection};
    use crate::write_ticket::current_validity::test_support::stored_evaluation;

    fn selected_id(selection: StoredWriteTicketSelection) -> Option<String> {
        match selection {
            StoredWriteTicketSelection::None => None,
            StoredWriteTicketSelection::Selected(ticket) => {
                Some(ticket.write_ticket_id().as_str().to_owned())
            }
        }
    }

    #[test]
    fn empty_candidates_select_nothing() {
        assert_eq!(
            select_stored_write_ticket(Vec::new()),
            StoredWriteTicketSelection::None
        );
    }

    #[test]
    fn active_status_has_priority_over_newer_terminal_tickets() {
        let selection = select_stored_write_ticket(vec![
            stored_evaluation("ticket-consumed", WriteTicketStatus::Consumed, 11),
            stored_evaluation("ticket-active", WriteTicketStatus::Active, 4),
            stored_evaluation("ticket-invalidated", WriteTicketStatus::Invalidated, 12),
            stored_evaluation("ticket-revoked", WriteTicketStatus::Revoked, 13),
        ]);

        assert_eq!(selected_id(selection).as_deref(), Some("ticket-active"));
    }

    #[test]
    fn equal_status_prefers_newer_basis_then_stable_identity() {
        let newer = select_stored_write_ticket(vec![
            stored_evaluation("ticket-a", WriteTicketStatus::Invalidated, 7),
            stored_evaluation("ticket-b", WriteTicketStatus::Invalidated, 8),
        ]);
        assert_eq!(selected_id(newer).as_deref(), Some("ticket-b"));

        let stable = select_stored_write_ticket(vec![
            stored_evaluation("ticket-b", WriteTicketStatus::Consumed, 8),
            stored_evaluation("ticket-a", WriteTicketStatus::Consumed, 8),
        ]);
        assert_eq!(selected_id(stable).as_deref(), Some("ticket-a"));
    }
}
