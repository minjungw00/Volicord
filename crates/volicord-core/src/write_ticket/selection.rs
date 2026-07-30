use std::cmp::Ordering;

use volicord_types::ids::WriteTicketId;
use volicord_types::values::WriteTicketStatus;

use super::current_validity::{ReusableStoredWriteTicket, StoredWriteTicketEvaluation};

#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompatibleWriteTicketSelection {
    None,
    One(Box<ReusableStoredWriteTicket>),
    Ambiguous(AmbiguousCompatibleWriteTickets),
}

#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AmbiguousCompatibleWriteTickets {
    write_ticket_ids: Box<[WriteTicketId]>,
}

impl AmbiguousCompatibleWriteTickets {
    fn new(
        first: WriteTicketId,
        second: WriteTicketId,
        remaining: impl IntoIterator<Item = WriteTicketId>,
    ) -> Self {
        let mut write_ticket_ids = vec![first, second];
        write_ticket_ids.extend(remaining);
        write_ticket_ids.sort();
        Self {
            write_ticket_ids: write_ticket_ids.into_boxed_slice(),
        }
    }

    pub(crate) fn write_ticket_ids(&self) -> &[WriteTicketId] {
        &self.write_ticket_ids
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PrepareWriteCandidateEvaluation {
    Incompatible,
    Compatible(Box<ReusableStoredWriteTicket>),
    StaleApproval(WriteTicketId),
    StaleWorkspace(WriteTicketId),
    StalePolicy(WriteTicketId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PrepareWriteCandidateSelection {
    pub(crate) compatibility: CompatibleWriteTicketSelection,
    pub(crate) stale_approval_ticket_ids: Vec<WriteTicketId>,
    pub(crate) stale_workspace_ticket_ids: Vec<WriteTicketId>,
    pub(crate) stale_policy_ticket_ids: Vec<WriteTicketId>,
}

pub(crate) fn select_prepare_write_candidates(
    candidates: Vec<PrepareWriteCandidateEvaluation>,
) -> PrepareWriteCandidateSelection {
    let mut compatible = Vec::new();
    let mut stale_approval_ticket_ids = Vec::new();
    let mut stale_workspace_ticket_ids = Vec::new();
    let mut stale_policy_ticket_ids = Vec::new();

    for candidate in candidates {
        match candidate {
            PrepareWriteCandidateEvaluation::Incompatible => {}
            PrepareWriteCandidateEvaluation::Compatible(ticket) => compatible.push(ticket),
            PrepareWriteCandidateEvaluation::StaleApproval(write_ticket_id) => {
                stale_approval_ticket_ids.push(write_ticket_id);
            }
            PrepareWriteCandidateEvaluation::StaleWorkspace(write_ticket_id) => {
                stale_workspace_ticket_ids.push(write_ticket_id);
            }
            PrepareWriteCandidateEvaluation::StalePolicy(write_ticket_id) => {
                stale_policy_ticket_ids.push(write_ticket_id);
            }
        }
    }

    let mut compatible = compatible.into_iter();
    let compatibility = match compatible.next() {
        None => CompatibleWriteTicketSelection::None,
        Some(first) => match compatible.next() {
            None => CompatibleWriteTicketSelection::One(first),
            Some(second) => {
                CompatibleWriteTicketSelection::Ambiguous(AmbiguousCompatibleWriteTickets::new(
                    first.write_ticket_id().clone(),
                    second.write_ticket_id().clone(),
                    compatible.map(|ticket| ticket.write_ticket_id().clone()),
                ))
            }
        },
    };

    stale_approval_ticket_ids.sort();
    stale_workspace_ticket_ids.sort();
    stale_policy_ticket_ids.sort();

    PrepareWriteCandidateSelection {
        compatibility,
        stale_approval_ticket_ids,
        stale_workspace_ticket_ids,
        stale_policy_ticket_ids,
    }
}

#[must_use]
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
                .basis_state_version()
                .cmp(&left.semantic_facts().basis_state_version())
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

    use super::{
        select_prepare_write_candidates, select_stored_write_ticket,
        CompatibleWriteTicketSelection, PrepareWriteCandidateEvaluation,
        StoredWriteTicketSelection,
    };
    use crate::write_ticket::current_validity::{
        test_support::stored_evaluation, ReusableStoredWriteTicket, StoredWriteTicketEvaluation,
    };

    fn compatible(write_ticket_id: &str) -> PrepareWriteCandidateEvaluation {
        PrepareWriteCandidateEvaluation::Compatible(Box::new(reusable(write_ticket_id)))
    }

    fn reusable(write_ticket_id: &str) -> ReusableStoredWriteTicket {
        match stored_evaluation(write_ticket_id, WriteTicketStatus::Active, 4) {
            StoredWriteTicketEvaluation::Reusable(ticket) => ticket,
            _ => unreachable!("an active stored evaluation is reusable"),
        }
    }

    fn one_id(selection: CompatibleWriteTicketSelection) -> String {
        match selection {
            CompatibleWriteTicketSelection::One(ticket) => {
                ticket.write_ticket_id().as_str().to_owned()
            }
            other => panic!("expected exactly one compatible ticket, got {other:?}"),
        }
    }

    fn ambiguous_ids(selection: CompatibleWriteTicketSelection) -> Vec<String> {
        match selection {
            CompatibleWriteTicketSelection::Ambiguous(ambiguous) => ambiguous
                .write_ticket_ids()
                .iter()
                .map(|write_ticket_id| write_ticket_id.as_str().to_owned())
                .collect(),
            other => panic!("expected ambiguous compatible tickets, got {other:?}"),
        }
    }

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
    fn no_active_prepare_write_candidates_select_no_compatible_ticket() {
        assert_eq!(
            select_prepare_write_candidates(Vec::new()).compatibility,
            CompatibleWriteTicketSelection::None
        );
    }

    #[test]
    fn active_but_incompatible_prepare_write_candidates_select_no_ticket() {
        let selection = select_prepare_write_candidates(vec![
            PrepareWriteCandidateEvaluation::Incompatible,
            PrepareWriteCandidateEvaluation::Incompatible,
        ]);

        assert_eq!(
            selection.compatibility,
            CompatibleWriteTicketSelection::None
        );
    }

    #[test]
    fn exactly_one_compatible_prepare_write_candidate_is_selected() {
        let selection = select_prepare_write_candidates(vec![compatible("ticket-only")]);

        assert_eq!(one_id(selection.compatibility), "ticket-only");
    }

    #[test]
    fn two_compatible_prepare_write_candidates_are_ambiguous() {
        let selection =
            select_prepare_write_candidates(vec![compatible("ticket-b"), compatible("ticket-a")]);

        assert_eq!(
            ambiguous_ids(selection.compatibility),
            vec!["ticket-a", "ticket-b"]
        );
    }

    #[test]
    fn more_than_two_compatible_prepare_write_candidates_are_ambiguous() {
        let selection = select_prepare_write_candidates(vec![
            compatible("ticket-c"),
            compatible("ticket-a"),
            compatible("ticket-b"),
        ]);

        assert_eq!(
            ambiguous_ids(selection.compatibility),
            vec!["ticket-a", "ticket-b", "ticket-c"]
        );
    }

    #[test]
    fn one_compatible_candidate_is_selected_among_incompatible_candidates() {
        let selection = select_prepare_write_candidates(vec![
            PrepareWriteCandidateEvaluation::Incompatible,
            compatible("ticket-compatible"),
            PrepareWriteCandidateEvaluation::Incompatible,
        ]);

        assert_eq!(one_id(selection.compatibility), "ticket-compatible");
    }

    #[test]
    fn ambiguity_diagnostics_are_sorted_without_selecting_a_candidate() {
        let selection = select_prepare_write_candidates(vec![
            compatible("ticket-z"),
            PrepareWriteCandidateEvaluation::Incompatible,
            compatible("ticket-a"),
            compatible("ticket-m"),
        ]);

        assert_eq!(
            ambiguous_ids(selection.compatibility),
            vec!["ticket-a", "ticket-m", "ticket-z"]
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
