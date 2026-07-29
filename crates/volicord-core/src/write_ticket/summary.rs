use volicord_types::ids::{RunId, TaskId, WriteTicketId};
use volicord_types::schema::{GuaranteeDisplay, StateRecordRef, WriteTicketStateSummary};
use volicord_types::values::{StateRecordKind, WriteTicketInvalidationReason, WriteTicketStatus};

use crate::record_refs::state_ref;

use super::current_validity::StoredWriteTicketEvaluation;
use super::planning::PlannedWriteTicket;
use super::read_model::WriteTicketEvidenceFacts;
use super::semantic::{planned_write_ticket_semantic_facts, WriteTicketSemanticFacts};

pub(crate) struct StoredWriteTicketSummaryInput<'a> {
    pub(crate) evaluated: &'a StoredWriteTicketEvaluation,
    pub(crate) state_version: u64,
    pub(crate) evidence: &'a WriteTicketEvidenceFacts,
    pub(crate) guarantee_display: Option<GuaranteeDisplay>,
}

pub(crate) struct PlannedWriteTicketSummaryInput<'a> {
    pub(crate) planned: &'a PlannedWriteTicket,
    pub(crate) state_version: u64,
    pub(crate) guarantee_display: Option<GuaranteeDisplay>,
}

struct WriteTicketSummaryFacts<'a> {
    write_ticket_id: &'a WriteTicketId,
    ticket: &'a WriteTicketSemanticFacts,
    status: WriteTicketStatus,
    invalidation: Option<WriteTicketInvalidationReason>,
    consumed_by_run_id: Option<&'a RunId>,
}

pub(crate) fn project_stored_write_ticket_summary(
    input: StoredWriteTicketSummaryInput<'_>,
) -> WriteTicketStateSummary {
    let StoredWriteTicketSummaryInput {
        evaluated,
        state_version,
        evidence,
        guarantee_display,
    } = input;
    project_summary(
        WriteTicketSummaryFacts {
            write_ticket_id: evaluated.write_ticket_id(),
            ticket: evaluated.semantic_facts(),
            status: evaluated.status(),
            invalidation: evaluated.invalidation(),
            consumed_by_run_id: evaluated.consumed_by_run_id(),
        },
        state_version,
        evidence,
        guarantee_display,
    )
}

pub(crate) fn project_planned_write_ticket_summary(
    input: PlannedWriteTicketSummaryInput<'_>,
) -> WriteTicketStateSummary {
    let PlannedWriteTicketSummaryInput {
        planned,
        state_version,
        guarantee_display,
    } = input;
    let ticket = planned_write_ticket_semantic_facts(planned);
    project_summary(
        WriteTicketSummaryFacts {
            write_ticket_id: planned.write_ticket_id(),
            ticket: &ticket,
            status: WriteTicketStatus::Active,
            invalidation: None,
            consumed_by_run_id: None,
        },
        state_version,
        &WriteTicketEvidenceFacts::default(),
        guarantee_display,
    )
}

fn project_summary(
    facts: WriteTicketSummaryFacts<'_>,
    state_version: u64,
    evidence: &WriteTicketEvidenceFacts,
    guarantee_display: Option<GuaranteeDisplay>,
) -> WriteTicketStateSummary {
    let task_id = &facts.ticket.validity_basis.task_id;
    let write_ticket_ref = Some(write_ticket_state_ref(
        facts.write_ticket_id,
        facts.ticket,
        task_id,
        state_version,
    ));
    let consumed_by_run_ref = facts.consumed_by_run_id.map(|run_id| {
        state_ref(
            StateRecordKind::Run,
            run_id.as_str(),
            &facts.ticket.project_id,
            Some(task_id),
            Some(state_version),
        )
    });
    WriteTicketStateSummary {
        status: facts.status,
        write_ticket_ref,
        basis_state_version: Some(facts.ticket.basis_state_version),
        validity_basis: Some(facts.ticket.validity_basis.clone()),
        invalidation_reason: facts.invalidation,
        idle_expires_at: facts.ticket.idle_expires_at.clone(),
        intended_paths: facts
            .ticket
            .attempt_scope
            .intended_paths
            .iter()
            .map(|path| path.as_str().to_owned())
            .collect(),
        consumed_by_run_ref,
        observation_refs: evidence.observation_refs.clone(),
        guarantee_display,
    }
}

fn write_ticket_state_ref(
    write_ticket_id: &WriteTicketId,
    ticket: &WriteTicketSemanticFacts,
    task_id: &TaskId,
    state_version: u64,
) -> StateRecordRef {
    state_ref(
        StateRecordKind::WriteTicket,
        write_ticket_id.as_str(),
        &ticket.project_id,
        Some(task_id),
        Some(state_version),
    )
}

#[cfg(test)]
mod tests {
    use volicord_types::values::{
        StateRecordKind, WriteTicketInvalidationReason, WriteTicketStatus,
    };

    use super::{project_stored_write_ticket_summary, StoredWriteTicketSummaryInput};
    use crate::record_refs::state_ref;
    use crate::write_ticket::current_validity::test_support::stored_evaluation;
    use crate::write_ticket::read_model::WriteTicketEvidenceFacts;

    #[test]
    fn stored_ticket_summary_maps_only_supplied_evaluated_and_evidence_facts() {
        let evaluated = stored_evaluation("ticket-summary", WriteTicketStatus::Consumed, 9);
        let task_id = evaluated.semantic_facts().validity_basis.task_id.clone();
        let project_id = evaluated.semantic_facts().project_id.clone();
        let observation_ref = state_ref(
            StateRecordKind::EvidenceObservation,
            "observation-summary",
            &project_id,
            Some(&task_id),
            Some(12),
        );
        let evidence = WriteTicketEvidenceFacts {
            observation_refs: vec![observation_ref.clone()],
        };

        let summary = project_stored_write_ticket_summary(StoredWriteTicketSummaryInput {
            evaluated: &evaluated,
            state_version: 12,
            evidence: &evidence,
            guarantee_display: None,
        });

        assert_eq!(summary.status, WriteTicketStatus::Consumed);
        assert_eq!(
            summary
                .write_ticket_ref
                .as_ref()
                .map(|reference| reference.record_id.as_str()),
            Some("ticket-summary")
        );
        assert_eq!(summary.basis_state_version, Some(9));
        assert_eq!(summary.invalidation_reason, None);
        assert_eq!(summary.intended_paths, vec!["src".to_owned()]);
        assert_eq!(
            summary
                .consumed_by_run_ref
                .as_ref()
                .map(|reference| reference.record_id.as_str()),
            Some("run-test")
        );
        assert_eq!(summary.observation_refs, vec![observation_ref]);
    }

    #[test]
    fn invalidation_reason_is_copied_from_evaluated_state() {
        let evaluated = stored_evaluation("ticket-invalidated", WriteTicketStatus::Invalidated, 9);

        let summary = project_stored_write_ticket_summary(StoredWriteTicketSummaryInput {
            evaluated: &evaluated,
            state_version: 12,
            evidence: &WriteTicketEvidenceFacts::default(),
            guarantee_display: None,
        });

        assert_eq!(
            summary.invalidation_reason,
            Some(WriteTicketInvalidationReason::ExplicitRevoke)
        );
        assert!(summary.consumed_by_run_ref.is_none());
    }
}
