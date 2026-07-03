use serde_json::Value;
use volicord_types::SummaryCard;

use crate::disclosure::{
    does_not_prove_line, AUTHORITY_RECORD_NON_GUARANTEE_TEXT,
    DETECTIVE_OBSERVATION_NON_GUARANTEE_TEXT, USER_CHANNEL_NON_GUARANTEE_TEXT,
};

pub(crate) const DIAGNOSTIC_SUMMARY_GUARANTEE: &str =
    "Local diagnostic observation; not OS enforcement, write prevention, actor attribution proof, correctness proof, test sufficiency proof, or review completion.";

pub(crate) const USER_CHANNEL_SUMMARY_GUARANTEE: &str =
    "Local User Channel view; listing does not record a judgment or prove close readiness.";

pub(crate) fn render_summary_card_text(card: &SummaryCard) -> String {
    format!(
        "Task: {}\nRecording: {}\nProfile: {}\nWrite Ticket: {}\nEvidence: {}\nUser Judgment: {}\nChanges: {}\nClose Status: {}\nTransport: {}\nNext: {}\n{}",
        card.task,
        card.recording,
        card.profile,
        card.write_ticket,
        card.evidence,
        card.user_judgment,
        card.changes,
        card.close_status,
        card.transport,
        card.next,
        does_not_prove_line(summary_card_non_guarantees(card)),
    )
}

fn summary_card_non_guarantees(card: &SummaryCard) -> &'static str {
    match card.guarantee.as_str() {
        DIAGNOSTIC_SUMMARY_GUARANTEE => DETECTIVE_OBSERVATION_NON_GUARANTEE_TEXT,
        USER_CHANNEL_SUMMARY_GUARANTEE => USER_CHANNEL_NON_GUARANTEE_TEXT,
        _ => AUTHORITY_RECORD_NON_GUARANTEE_TEXT,
    }
}

pub(crate) fn summary_card_from_response(value: &Value) -> Option<SummaryCard> {
    serde_json::from_value(value.get("summary_card")?.clone()).ok()
}

pub(crate) fn count_state_text(label: &str, count: usize) -> String {
    if count == 0 {
        "none".to_owned()
    } else {
        format!("{label} ({count})")
    }
}
