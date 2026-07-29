use volicord_types::ids::TaskId;
use volicord_types::schema::{GuaranteeDisplay, StateRecordRef, WriteTicketStateSummary};
use volicord_types::values::StateRecordKind;

use crate::record_refs::state_ref;

use super::current_validity::EvaluatedWriteTicket;
use super::read_model::WriteTicketEvidenceFacts;
use super::semantic::WriteTicketEvaluationIdentity;

pub(crate) struct WriteTicketSummaryInput<'a> {
    pub(crate) evaluated: &'a EvaluatedWriteTicket,
    pub(crate) state_version: u64,
    pub(crate) evidence: &'a WriteTicketEvidenceFacts,
    pub(crate) guarantee_display: Option<GuaranteeDisplay>,
}

pub(crate) fn project_write_ticket_summary(
    input: WriteTicketSummaryInput<'_>,
) -> WriteTicketStateSummary {
    let WriteTicketSummaryInput {
        evaluated,
        state_version,
        evidence,
        guarantee_display,
    } = input;
    let task_id = &evaluated.ticket.validity_basis.task_id;
    let write_ticket_ref = match &evaluated.identity {
        WriteTicketEvaluationIdentity::Planned { write_ticket_id } => {
            write_ticket_id.as_ref().map(|write_ticket_id| {
                write_ticket_state_ref(write_ticket_id.as_str(), evaluated, task_id, state_version)
            })
        }
        WriteTicketEvaluationIdentity::Stored { write_ticket_id } => Some(write_ticket_state_ref(
            write_ticket_id.as_str(),
            evaluated,
            task_id,
            state_version,
        )),
    };
    let consumed_by_run_ref = evaluated.consumed_by_run_id.as_ref().map(|run_id| {
        state_ref(
            StateRecordKind::Run,
            run_id.as_str(),
            &evaluated.ticket.project_id,
            Some(task_id),
            Some(state_version),
        )
    });
    WriteTicketStateSummary {
        status: evaluated.effective_status,
        write_ticket_ref,
        basis_state_version: Some(evaluated.ticket.basis_state_version),
        validity_basis: Some(evaluated.ticket.validity_basis.clone()),
        invalidation_reason: evaluated.invalidation,
        idle_expires_at: evaluated.ticket.idle_expires_at.clone(),
        intended_paths: evaluated
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
    write_ticket_id: &str,
    evaluated: &EvaluatedWriteTicket,
    task_id: &TaskId,
    state_version: u64,
) -> StateRecordRef {
    state_ref(
        StateRecordKind::WriteTicket,
        write_ticket_id,
        &evaluated.ticket.project_id,
        Some(task_id),
        Some(state_version),
    )
}

#[cfg(test)]
mod tests {
    use volicord_types::ids::{RunId, WriteTicketId};
    use volicord_types::values::{
        StateRecordKind, WriteTicketInvalidationReason, WriteTicketStatus,
    };

    use super::{project_write_ticket_summary, WriteTicketSummaryInput};
    use crate::record_refs::state_ref;
    use crate::write_ticket::read_model::WriteTicketEvidenceFacts;
    use crate::write_ticket::semantic::{
        test_support::evaluated_ticket, WriteTicketEvaluationIdentity,
    };

    #[test]
    fn stored_ticket_summary_maps_only_supplied_evaluated_and_evidence_facts() {
        let mut evaluated = evaluated_ticket("ticket-summary", WriteTicketStatus::Consumed, 9);
        evaluated.consumed_by_run_id = Some(RunId::new("run-summary"));
        let observation_ref = state_ref(
            StateRecordKind::EvidenceObservation,
            "observation-summary",
            &evaluated.ticket.project_id,
            Some(&evaluated.ticket.validity_basis.task_id),
            Some(12),
        );
        let evidence = WriteTicketEvidenceFacts {
            observation_refs: vec![observation_ref.clone()],
        };

        let summary = project_write_ticket_summary(WriteTicketSummaryInput {
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
            Some("run-summary")
        );
        assert_eq!(summary.observation_refs, vec![observation_ref]);
    }

    #[test]
    fn invalidation_reason_is_copied_from_evaluated_state() {
        let mut evaluated =
            evaluated_ticket("ticket-invalidated", WriteTicketStatus::Invalidated, 9);
        evaluated.invalidation = Some(WriteTicketInvalidationReason::ExplicitRevoke);

        let summary = project_write_ticket_summary(WriteTicketSummaryInput {
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

    #[test]
    fn identityless_planned_ticket_has_no_persisted_record_reference() {
        let mut evaluated = evaluated_ticket("unused", WriteTicketStatus::Active, 7);
        evaluated.identity = WriteTicketEvaluationIdentity::Planned {
            write_ticket_id: None,
        };

        let summary = project_write_ticket_summary(WriteTicketSummaryInput {
            evaluated: &evaluated,
            state_version: 8,
            evidence: &WriteTicketEvidenceFacts::default(),
            guarantee_display: None,
        });

        assert!(summary.write_ticket_ref.is_none());

        evaluated.identity = WriteTicketEvaluationIdentity::Planned {
            write_ticket_id: Some(WriteTicketId::new("ticket-planned")),
        };
        let summary = project_write_ticket_summary(WriteTicketSummaryInput {
            evaluated: &evaluated,
            state_version: 8,
            evidence: &WriteTicketEvidenceFacts::default(),
            guarantee_display: None,
        });
        assert_eq!(
            summary
                .write_ticket_ref
                .as_ref()
                .map(|reference| reference.record_id.as_str()),
            Some("ticket-planned")
        );
    }
}
