use volicord_types::ids::{RunId, WriteTicketId};
use volicord_types::values::{WriteTicketInvalidationReason, WriteTicketStatus};

use super::approval::WriteTicketApprovalAssessment;
use super::policy::write_ticket_is_idle_expired;
use super::read_model::WriteTicketCurrentFacts;
use super::semantic::{StoredWriteTicketFacts, WriteTicketSemanticFacts};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WriteTicketAuthorityState {
    NotApplicable,
    Current,
    WriteAuthorityChanged,
    PendingPolicyReevaluation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StoredWriteTicketState {
    write_ticket_id: WriteTicketId,
    ticket: WriteTicketSemanticFacts,
}

/// A physically active stored ticket that still requires current semantic facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActiveStoredWriteTicketCandidate {
    stored: StoredWriteTicketState,
}

impl ActiveStoredWriteTicketCandidate {
    pub(crate) fn write_ticket_id(&self) -> &WriteTicketId {
        &self.stored.write_ticket_id
    }

    pub(crate) fn semantic_facts(&self) -> &WriteTicketSemanticFacts {
        &self.stored.ticket
    }
}

/// A stored ticket proven reusable against current authority and approval facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReusableStoredWriteTicket {
    stored: StoredWriteTicketState,
}

impl ReusableStoredWriteTicket {
    pub fn write_ticket_id(&self) -> &WriteTicketId {
        &self.stored.write_ticket_id
    }

    pub(crate) fn semantic_facts(&self) -> &WriteTicketSemanticFacts {
        &self.stored.ticket
    }
}

/// A stored ticket whose persisted or effective current state is invalidated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidatedStoredWriteTicket {
    stored: StoredWriteTicketState,
    invalidation: WriteTicketInvalidationReason,
    authority: WriteTicketAuthorityState,
}

impl InvalidatedStoredWriteTicket {
    pub fn write_ticket_id(&self) -> &WriteTicketId {
        &self.stored.write_ticket_id
    }

    pub fn invalidation(&self) -> WriteTicketInvalidationReason {
        self.invalidation
    }

    pub(crate) fn authority(&self) -> WriteTicketAuthorityState {
        self.authority
    }

    pub(crate) fn semantic_facts(&self) -> &WriteTicketSemanticFacts {
        &self.stored.ticket
    }
}

/// A stored ticket consumed by one recorded Run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumedStoredWriteTicket {
    stored: StoredWriteTicketState,
    consumed_by_run_id: RunId,
}

impl ConsumedStoredWriteTicket {
    pub fn write_ticket_id(&self) -> &WriteTicketId {
        &self.stored.write_ticket_id
    }

    pub fn consumed_by_run_id(&self) -> &RunId {
        &self.consumed_by_run_id
    }

    pub(crate) fn semantic_facts(&self) -> &WriteTicketSemanticFacts {
        &self.stored.ticket
    }
}

/// A stored ticket explicitly revoked in persisted state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevokedStoredWriteTicket {
    stored: StoredWriteTicketState,
    invalidation: WriteTicketInvalidationReason,
}

impl RevokedStoredWriteTicket {
    pub fn write_ticket_id(&self) -> &WriteTicketId {
        &self.stored.write_ticket_id
    }

    pub fn invalidation(&self) -> WriteTicketInvalidationReason {
        self.invalidation
    }

    pub(crate) fn semantic_facts(&self) -> &WriteTicketSemanticFacts {
        &self.stored.ticket
    }
}

/// Stored-only evaluation state consumed by persisted selection and projection.
#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoredWriteTicketEvaluation {
    Reusable(ReusableStoredWriteTicket),
    Invalidated(InvalidatedStoredWriteTicket),
    Consumed(ConsumedStoredWriteTicket),
    Revoked(RevokedStoredWriteTicket),
}

impl StoredWriteTicketEvaluation {
    pub fn write_ticket_id(&self) -> &WriteTicketId {
        match self {
            Self::Reusable(ticket) => ticket.write_ticket_id(),
            Self::Invalidated(ticket) => ticket.write_ticket_id(),
            Self::Consumed(ticket) => ticket.write_ticket_id(),
            Self::Revoked(ticket) => ticket.write_ticket_id(),
        }
    }

    pub fn invalidation(&self) -> Option<WriteTicketInvalidationReason> {
        match self {
            Self::Reusable(_) | Self::Consumed(_) => None,
            Self::Invalidated(ticket) => Some(ticket.invalidation()),
            Self::Revoked(ticket) => Some(ticket.invalidation()),
        }
    }

    pub fn status(&self) -> WriteTicketStatus {
        match self {
            Self::Reusable(_) => WriteTicketStatus::Active,
            Self::Invalidated(_) => WriteTicketStatus::Invalidated,
            Self::Consumed(_) => WriteTicketStatus::Consumed,
            Self::Revoked(_) => WriteTicketStatus::Revoked,
        }
    }

    pub(crate) fn semantic_facts(&self) -> &WriteTicketSemanticFacts {
        match self {
            Self::Reusable(ticket) => ticket.semantic_facts(),
            Self::Invalidated(ticket) => ticket.semantic_facts(),
            Self::Consumed(ticket) => ticket.semantic_facts(),
            Self::Revoked(ticket) => ticket.semantic_facts(),
        }
    }

    pub(crate) fn consumed_by_run_id(&self) -> Option<&RunId> {
        match self {
            Self::Consumed(ticket) => Some(ticket.consumed_by_run_id()),
            Self::Reusable(_) | Self::Invalidated(_) | Self::Revoked(_) => None,
        }
    }

    pub(crate) fn as_reusable(&self) -> Option<&ReusableStoredWriteTicket> {
        match self {
            Self::Reusable(ticket) => Some(ticket),
            Self::Invalidated(_) | Self::Consumed(_) | Self::Revoked(_) => None,
        }
    }

    pub(crate) fn as_consumed(&self) -> Option<&ConsumedStoredWriteTicket> {
        match self {
            Self::Consumed(ticket) => Some(ticket),
            Self::Reusable(_) | Self::Invalidated(_) | Self::Revoked(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TerminalStoredWriteTicketEvaluation {
    Invalidated(InvalidatedStoredWriteTicket),
    Consumed(ConsumedStoredWriteTicket),
    Revoked(RevokedStoredWriteTicket),
}

impl From<TerminalStoredWriteTicketEvaluation> for StoredWriteTicketEvaluation {
    fn from(evaluation: TerminalStoredWriteTicketEvaluation) -> Self {
        match evaluation {
            TerminalStoredWriteTicketEvaluation::Invalidated(ticket) => Self::Invalidated(ticket),
            TerminalStoredWriteTicketEvaluation::Consumed(ticket) => Self::Consumed(ticket),
            TerminalStoredWriteTicketEvaluation::Revoked(ticket) => Self::Revoked(ticket),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActiveStoredWriteTicketEvaluation {
    Reusable(ReusableStoredWriteTicket),
    Invalidated(InvalidatedStoredWriteTicket),
}

impl ActiveStoredWriteTicketEvaluation {
    pub(crate) fn semantic_facts(&self) -> &WriteTicketSemanticFacts {
        match self {
            Self::Reusable(ticket) => ticket.semantic_facts(),
            Self::Invalidated(ticket) => ticket.semantic_facts(),
        }
    }
}

impl From<ActiveStoredWriteTicketEvaluation> for StoredWriteTicketEvaluation {
    fn from(evaluation: ActiveStoredWriteTicketEvaluation) -> Self {
        match evaluation {
            ActiveStoredWriteTicketEvaluation::Reusable(ticket) => Self::Reusable(ticket),
            ActiveStoredWriteTicketEvaluation::Invalidated(ticket) => Self::Invalidated(ticket),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StoredTicketPreEvaluation {
    Complete(TerminalStoredWriteTicketEvaluation),
    NeedsCurrentFacts(ActiveStoredWriteTicketCandidate),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StoredWriteTicketStateError {
    ActiveLifecycleDetails,
    MissingInvalidationReason,
    MissingConsumingRun,
    UnexpectedConsumingRun,
}

/// Separates persisted terminal lifecycle from active candidates before current evaluation.
pub(crate) fn pre_evaluate_stored_write_ticket(
    ticket: StoredWriteTicketFacts,
    observed_at: &volicord_types::values::UtcTimestamp,
) -> Result<StoredTicketPreEvaluation, StoredWriteTicketStateError> {
    let StoredWriteTicketFacts {
        write_ticket_id,
        ticket,
        status,
        invalidation_reason,
        consumed_by_run_id,
    } = ticket;
    let stored = StoredWriteTicketState {
        write_ticket_id,
        ticket,
    };
    match status {
        WriteTicketStatus::Active => {
            if invalidation_reason.is_some() || consumed_by_run_id.is_some() {
                return Err(StoredWriteTicketStateError::ActiveLifecycleDetails);
            }
            if write_ticket_is_idle_expired(stored.ticket.idle_expires_at.as_ref(), observed_at) {
                Ok(StoredTicketPreEvaluation::Complete(
                    TerminalStoredWriteTicketEvaluation::Invalidated(
                        InvalidatedStoredWriteTicket {
                            stored,
                            invalidation: WriteTicketInvalidationReason::IdleTimeout,
                            authority: WriteTicketAuthorityState::NotApplicable,
                        },
                    ),
                ))
            } else {
                Ok(StoredTicketPreEvaluation::NeedsCurrentFacts(
                    ActiveStoredWriteTicketCandidate { stored },
                ))
            }
        }
        WriteTicketStatus::Invalidated => {
            if consumed_by_run_id.is_some() {
                return Err(StoredWriteTicketStateError::UnexpectedConsumingRun);
            }
            let Some(invalidation) = invalidation_reason else {
                return Err(StoredWriteTicketStateError::MissingInvalidationReason);
            };
            Ok(StoredTicketPreEvaluation::Complete(
                TerminalStoredWriteTicketEvaluation::Invalidated(InvalidatedStoredWriteTicket {
                    stored,
                    invalidation,
                    authority: WriteTicketAuthorityState::NotApplicable,
                }),
            ))
        }
        WriteTicketStatus::Consumed => {
            if invalidation_reason.is_some() {
                return Err(StoredWriteTicketStateError::UnexpectedConsumingRun);
            }
            let Some(consumed_by_run_id) = consumed_by_run_id else {
                return Err(StoredWriteTicketStateError::MissingConsumingRun);
            };
            Ok(StoredTicketPreEvaluation::Complete(
                TerminalStoredWriteTicketEvaluation::Consumed(ConsumedStoredWriteTicket {
                    stored,
                    consumed_by_run_id,
                }),
            ))
        }
        WriteTicketStatus::Revoked => {
            if consumed_by_run_id.is_some() {
                return Err(StoredWriteTicketStateError::UnexpectedConsumingRun);
            }
            let Some(invalidation) = invalidation_reason else {
                return Err(StoredWriteTicketStateError::MissingInvalidationReason);
            };
            Ok(StoredTicketPreEvaluation::Complete(
                TerminalStoredWriteTicketEvaluation::Revoked(RevokedStoredWriteTicket {
                    stored,
                    invalidation,
                }),
            ))
        }
    }
}

pub(crate) fn evaluate_active_candidate(
    candidate: ActiveStoredWriteTicketCandidate,
    current: &WriteTicketCurrentFacts,
    approval: WriteTicketApprovalAssessment,
) -> ActiveStoredWriteTicketEvaluation {
    let basis = &candidate.stored.ticket.validity_basis;
    if basis.write_authority_fingerprint != current.workflow.write_authority_fingerprint {
        return ActiveStoredWriteTicketEvaluation::Invalidated(InvalidatedStoredWriteTicket {
            stored: candidate.stored,
            invalidation: WriteTicketInvalidationReason::ExplicitRevoke,
            authority: WriteTicketAuthorityState::WriteAuthorityChanged,
        });
    }
    if current.task.pending_policy_reevaluation {
        return ActiveStoredWriteTicketEvaluation::Invalidated(InvalidatedStoredWriteTicket {
            stored: candidate.stored,
            invalidation: WriteTicketInvalidationReason::ExplicitRevoke,
            authority: WriteTicketAuthorityState::PendingPolicyReevaluation,
        });
    }

    if matches!(approval, WriteTicketApprovalAssessment::Changed { .. }) {
        return ActiveStoredWriteTicketEvaluation::Invalidated(InvalidatedStoredWriteTicket {
            stored: candidate.stored,
            invalidation: WriteTicketInvalidationReason::ApprovalBasisChanged,
            authority: WriteTicketAuthorityState::Current,
        });
    }

    ActiveStoredWriteTicketEvaluation::Reusable(ReusableStoredWriteTicket {
        stored: candidate.stored,
    })
}

pub(crate) fn project_stored_write_ticket_consumption(
    ticket: &ReusableStoredWriteTicket,
    run_id: RunId,
) -> StoredWriteTicketEvaluation {
    StoredWriteTicketEvaluation::Consumed(ConsumedStoredWriteTicket {
        stored: ticket.stored.clone(),
        consumed_by_run_id: run_id,
    })
}

#[cfg(test)]
pub(crate) mod test_support {
    use volicord_types::ids::{RunId, WriteTicketId};
    use volicord_types::values::{WriteTicketInvalidationReason, WriteTicketStatus};

    use super::{
        ConsumedStoredWriteTicket, InvalidatedStoredWriteTicket, ReusableStoredWriteTicket,
        RevokedStoredWriteTicket, StoredWriteTicketEvaluation, StoredWriteTicketState,
        WriteTicketAuthorityState,
    };
    use crate::write_ticket::semantic::test_support::semantic_facts;

    pub(crate) fn stored_evaluation(
        write_ticket_id: &str,
        status: WriteTicketStatus,
        basis_state_version: u64,
    ) -> StoredWriteTicketEvaluation {
        let stored = StoredWriteTicketState {
            write_ticket_id: WriteTicketId::new(write_ticket_id),
            ticket: semantic_facts(basis_state_version),
        };
        match status {
            WriteTicketStatus::Active => {
                StoredWriteTicketEvaluation::Reusable(ReusableStoredWriteTicket { stored })
            }
            WriteTicketStatus::Invalidated => {
                StoredWriteTicketEvaluation::Invalidated(InvalidatedStoredWriteTicket {
                    stored,
                    invalidation: WriteTicketInvalidationReason::ExplicitRevoke,
                    authority: WriteTicketAuthorityState::NotApplicable,
                })
            }
            WriteTicketStatus::Consumed => {
                StoredWriteTicketEvaluation::Consumed(ConsumedStoredWriteTicket {
                    stored,
                    consumed_by_run_id: RunId::new("run-test"),
                })
            }
            WriteTicketStatus::Revoked => {
                StoredWriteTicketEvaluation::Revoked(RevokedStoredWriteTicket {
                    stored,
                    invalidation: WriteTicketInvalidationReason::ExplicitRevoke,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use volicord_types::values::{
        TaskControlLevel, WriteTicketInvalidationReason, WriteTicketStatus,
    };

    use super::{
        evaluate_active_candidate, pre_evaluate_stored_write_ticket,
        ActiveStoredWriteTicketEvaluation, StoredTicketPreEvaluation, StoredWriteTicketEvaluation,
        TerminalStoredWriteTicketEvaluation, WriteTicketAuthorityState,
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
    fn terminal_statuses_and_idle_expiry_do_not_require_current_facts() {
        let mut invalidated = stored_facts("ticket-invalidated", WriteTicketStatus::Invalidated, 7);
        invalidated.invalidation_reason = Some(WriteTicketInvalidationReason::ApprovalBasisChanged);
        let evaluated =
            pre_evaluate_stored_write_ticket(invalidated, &timestamp("2026-07-29T00:05:00Z"));
        let Ok(StoredTicketPreEvaluation::Complete(
            TerminalStoredWriteTicketEvaluation::Invalidated(invalidated),
        )) = evaluated
        else {
            panic!("persisted invalidated ticket should be terminal");
        };
        assert_eq!(invalidated.write_ticket_id().as_str(), "ticket-invalidated");
        assert_eq!(
            invalidated.invalidation(),
            WriteTicketInvalidationReason::ApprovalBasisChanged
        );

        let mut consumed = stored_facts("ticket-consumed", WriteTicketStatus::Consumed, 7);
        consumed.consumed_by_run_id = Some(volicord_types::ids::RunId::new("run-consuming"));
        let evaluated =
            pre_evaluate_stored_write_ticket(consumed, &timestamp("2026-07-29T00:05:00Z"));
        let Ok(StoredTicketPreEvaluation::Complete(TerminalStoredWriteTicketEvaluation::Consumed(
            consumed,
        ))) = evaluated
        else {
            panic!("persisted consumed ticket should be terminal");
        };
        assert_eq!(consumed.write_ticket_id().as_str(), "ticket-consumed");
        assert_eq!(consumed.consumed_by_run_id().as_str(), "run-consuming");

        let mut revoked = stored_facts("ticket-revoked", WriteTicketStatus::Revoked, 7);
        revoked.invalidation_reason = Some(WriteTicketInvalidationReason::ExplicitRevoke);
        let evaluated =
            pre_evaluate_stored_write_ticket(revoked, &timestamp("2026-07-29T00:05:00Z"));
        let Ok(StoredTicketPreEvaluation::Complete(TerminalStoredWriteTicketEvaluation::Revoked(
            revoked,
        ))) = evaluated
        else {
            panic!("persisted revoked ticket should be terminal");
        };
        assert_eq!(revoked.write_ticket_id().as_str(), "ticket-revoked");
        assert_eq!(
            revoked.invalidation(),
            WriteTicketInvalidationReason::ExplicitRevoke
        );

        let active = stored_facts("ticket-expired", WriteTicketStatus::Active, 7);
        let evaluated =
            pre_evaluate_stored_write_ticket(active, &timestamp("2026-07-29T00:15:00Z"));
        let Ok(StoredTicketPreEvaluation::Complete(terminal)) = evaluated else {
            panic!("expired active ticket should become a terminal evaluation");
        };
        let stored: StoredWriteTicketEvaluation = terminal.into();
        assert_eq!(stored.status(), WriteTicketStatus::Invalidated);
        assert_eq!(
            stored.invalidation(),
            Some(WriteTicketInvalidationReason::IdleTimeout)
        );
        assert_eq!(stored.write_ticket_id().as_str(), "ticket-expired");
    }

    fn active_candidate(write_ticket_id: &str) -> super::ActiveStoredWriteTicketCandidate {
        let evaluated = pre_evaluate_stored_write_ticket(
            stored_facts(write_ticket_id, WriteTicketStatus::Active, 7),
            &timestamp("2026-07-29T00:05:00Z"),
        );
        let Ok(StoredTicketPreEvaluation::NeedsCurrentFacts(candidate)) = evaluated else {
            panic!("active test ticket should require current facts");
        };
        candidate
    }

    #[test]
    fn authority_change_and_pending_reevaluation_have_typed_results() {
        let mut changed = current_facts();
        changed.workflow.write_authority_fingerprint = format!("sha256:{}", "1".repeat(64));
        let evaluated = evaluate_active_candidate(
            active_candidate("ticket-changed"),
            &changed,
            WriteTicketApprovalAssessment::NotRequired,
        );
        let ActiveStoredWriteTicketEvaluation::Invalidated(evaluated) = evaluated else {
            panic!("changed authority should invalidate an active ticket");
        };
        assert_eq!(
            evaluated.authority(),
            WriteTicketAuthorityState::WriteAuthorityChanged
        );
        assert_eq!(
            evaluated.invalidation(),
            WriteTicketInvalidationReason::ExplicitRevoke
        );

        let mut pending = current_facts();
        pending.task.pending_policy_reevaluation = true;
        let evaluated = evaluate_active_candidate(
            active_candidate("ticket-pending"),
            &pending,
            WriteTicketApprovalAssessment::NotRequired,
        );
        let ActiveStoredWriteTicketEvaluation::Invalidated(evaluated) = evaluated else {
            panic!("pending authority should invalidate an active ticket");
        };
        assert_eq!(
            evaluated.authority(),
            WriteTicketAuthorityState::PendingPolicyReevaluation
        );
    }

    #[test]
    fn semantic_approval_assessment_drives_current_status() {
        let changed = evaluate_active_candidate(
            active_candidate("ticket-sensitive"),
            &current_facts(),
            WriteTicketApprovalAssessment::Changed {
                reason: ApprovalBasisChangeReason::NoCurrentResolution,
            },
        );
        let ActiveStoredWriteTicketEvaluation::Invalidated(changed) = changed else {
            panic!("changed approval should invalidate an active ticket");
        };
        assert_eq!(
            changed.invalidation(),
            WriteTicketInvalidationReason::ApprovalBasisChanged
        );
        let current = evaluate_active_candidate(
            active_candidate("ticket-current"),
            &current_facts(),
            WriteTicketApprovalAssessment::NotRequired,
        );
        let ActiveStoredWriteTicketEvaluation::Reusable(current) = current else {
            panic!("current active ticket should be reusable");
        };
        assert_eq!(current.write_ticket_id().as_str(), "ticket-current");
    }
}
