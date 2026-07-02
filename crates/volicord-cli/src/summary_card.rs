use serde_json::Value;
use volicord_types::SummaryCard;

pub(crate) const DIAGNOSTIC_SUMMARY_GUARANTEE: &str =
    "Local diagnostic observation; not OS enforcement, write prevention, actor attribution proof, correctness proof, test sufficiency proof, or review completion.";

pub(crate) const USER_CHANNEL_SUMMARY_GUARANTEE: &str =
    "Local User Channel view; listing does not record a judgment or prove close readiness.";

pub(crate) fn render_summary_card_text(card: &SummaryCard) -> String {
    format!(
        "Task: {}\nRecording: {}\nProfile: {}\nWrite Ticket: {}\nEvidence: {}\nUser Judgment: {}\nChanges: {}\nClose Status: {}\nTransport: {}\nNext: {}\nGuarantee: {}\n",
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
        card.guarantee,
    )
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
