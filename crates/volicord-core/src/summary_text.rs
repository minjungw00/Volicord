use crate::pipeline::VerifiedInvocationContext;
use volicord_store::core_pipeline::TaskRecord;
use volicord_types::schema::{
    EvidenceGateSummary, GuaranteeDisplay, NextActionSummary, SummaryCard, WriteTicketStateSummary,
};
use volicord_types::values::{ActorSource, CloseState, GuaranteeLevel, StatusCloseState};

pub(crate) struct SummaryCardInput<'a> {
    pub(crate) task: Option<&'a TaskRecord>,
    pub(crate) recording: &'a str,
    pub(crate) profile: Option<String>,
    pub(crate) write_ticket: String,
    pub(crate) evidence: String,
    pub(crate) pending_user_actions: usize,
    pub(crate) changes: String,
    pub(crate) close_status: String,
    pub(crate) verified_invocation: &'a VerifiedInvocationContext,
    pub(crate) next_action: Option<&'a NextActionSummary>,
}

pub(crate) fn summary_card(input: SummaryCardInput<'_>) -> SummaryCard {
    let next = input
        .next_action
        .map(next_action_label)
        .unwrap_or_else(|| "none".to_owned());
    SummaryCard {
        task: task_summary_text(input.task),
        recording: input.recording.to_owned(),
        profile: input.profile.unwrap_or_else(|| "not_selected".to_owned()),
        write_ticket: input.write_ticket,
        evidence: input.evidence,
        user_action: count_state_text("pending", input.pending_user_actions),
        changes: input.changes,
        close_status: input.close_status,
        transport: transport_summary(input.verified_invocation),
        next,
        next_action: input.next_action.cloned(),
        guarantee: AUTHORITY_RECORD_SUMMARY_GUARANTEE.to_owned(),
    }
}

const AUTHORITY_RECORD_SUMMARY_GUARANTEE: &str =
    "Local authority record; not OS enforcement, correctness proof, test sufficiency proof, or review completion.";

fn task_summary_text(task: Option<&TaskRecord>) -> String {
    task.map(|task| format!("selected ({})", task.lifecycle_phase.as_str()))
        .unwrap_or_else(|| "none".to_owned())
}

pub(crate) fn profile_summary_text(guarantee_display: Option<&GuaranteeDisplay>) -> Option<String> {
    guarantee_display.map(|display| match display.level {
        GuaranteeLevel::Cooperative => "record".to_owned(),
    })
}

pub(crate) fn write_ticket_summary_text(
    selected: bool,
    summary: Option<&WriteTicketStateSummary>,
) -> String {
    if !selected {
        return "not_selected".to_owned();
    }
    summary
        .map(|summary| summary.status.as_str())
        .unwrap_or("none")
        .to_owned()
}

pub(crate) fn evidence_gate_summary_text(
    selected: bool,
    summary: Option<&EvidenceGateSummary>,
) -> String {
    if !selected {
        return "not_selected".to_owned();
    }
    summary
        .map(|summary| summary.state.as_str())
        .unwrap_or("none")
        .to_owned()
}

pub(crate) fn close_state_summary_text(
    selected: bool,
    close_state: Option<StatusCloseState>,
) -> String {
    if !selected {
        return "not_selected".to_owned();
    }
    close_state
        .map(StatusCloseState::as_str)
        .unwrap_or("none")
        .to_owned()
}

pub(crate) fn close_state_text(close_state: CloseState) -> &'static str {
    close_state.as_str()
}

pub(crate) fn changes_summary_text(selected: bool, unresolved_count: u64) -> String {
    if !selected {
        return "not_selected".to_owned();
    }
    count_state_text("unresolved", unresolved_count as usize)
}

fn count_state_text(label: &str, count: usize) -> String {
    if count == 0 {
        "none".to_owned()
    } else {
        format!("{label} ({count})")
    }
}

fn next_action_label(action: &NextActionSummary) -> String {
    if !action.label.trim().is_empty() {
        action.label.clone()
    } else {
        action
            .blocking_question
            .clone()
            .unwrap_or_else(|| "none".to_owned())
    }
}

fn transport_summary(verified_invocation: &VerifiedInvocationContext) -> String {
    match &verified_invocation.actor_source {
        ActorSource::AgentConnection(_) => "Agent Connection".to_owned(),
        ActorSource::LocalUser => "User Channel".to_owned(),
        ActorSource::System => "system".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use volicord_types::schema::EvidenceGateSummary;
    use volicord_types::values::EvidenceGateState;

    #[test]
    fn evidence_text_projects_the_selected_policy_result_without_reevaluating_it() {
        let summary = EvidenceGateSummary {
            state: EvidenceGateState::Stale,
        };
        assert_eq!(evidence_gate_summary_text(true, Some(&summary)), "stale");
        assert_eq!(
            evidence_gate_summary_text(false, Some(&summary)),
            "not_selected"
        );
    }
}
