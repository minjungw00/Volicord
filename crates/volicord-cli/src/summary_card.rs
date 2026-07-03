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

pub(crate) fn render_coverage_summary_text(value: &Value) -> Option<String> {
    let coverage = value.get("coverage_summary")?;
    let profile = coverage_text(coverage.get("active_profile"));
    let host_hook = coverage_text(coverage.get("host_hook_state"));
    let session_watcher = coverage_text(coverage.get("session_watcher_state"));
    let started = coverage_text(coverage.get("coverage_started_at"));
    let last_snapshot = coverage_text(coverage.get("last_snapshot_at"));
    let unresolved = coverage
        .get("unresolved_unrecorded_change_count")
        .and_then(Value::as_u64)
        .map(|count| count.to_string())
        .unwrap_or_else(|| "unknown".to_owned());
    let non_guarantees = coverage
        .get("non_guarantees")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(coverage_non_guarantee_text)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "not listed".to_owned());

    Some(format!(
        "Coverage: profile={profile}; host_hook={host_hook}; session_watcher={session_watcher}; started={started}; last_snapshot={last_snapshot}; unresolved_unrecorded_changes={unresolved}\nCoverage does not guarantee: {non_guarantees}\n"
    ))
}

fn coverage_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) if !text.is_empty() => text.to_owned(),
        Some(Value::Null) => "none".to_owned(),
        Some(value) => value.to_string(),
        None => "unknown".to_owned(),
    }
}

fn coverage_non_guarantee_text(value: &str) -> &'static str {
    match value {
        "NotActorAttributionProof" => "actor identity proof",
        "NotFullFilesystemMonitoring" => "full filesystem monitoring",
        "NotFullWritePrevention" => "write prevention",
        _ => "listed non-guarantee",
    }
}

pub(crate) fn count_state_text(label: &str, count: usize) -> String {
    if count == 0 {
        "none".to_owned()
    } else {
        format!("{label} ({count})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn coverage_summary_text_reports_observation_limits() {
        let value = json!({
            "coverage_summary": {
                "active_profile": "detective",
                "host_hook_state": "observed",
                "session_watcher_state": "degraded",
                "coverage_started_at": "2026-06-30T00:03:00Z",
                "last_snapshot_at": "2026-06-30T00:04:00Z",
                "unresolved_unrecorded_change_count": 2,
                "non_guarantees": [
                    "NotActorAttributionProof",
                    "NotFullFilesystemMonitoring",
                    "NotFullWritePrevention"
                ]
            }
        });

        let text = render_coverage_summary_text(&value).expect("coverage line should render");

        assert!(text.contains("profile=detective"));
        assert!(text.contains("session_watcher=degraded"));
        assert!(text.contains("unresolved_unrecorded_changes=2"));
        assert!(text.contains("actor identity proof"));
        assert!(text.contains("full filesystem monitoring"));
        assert!(text.contains("write prevention"));
    }
}
