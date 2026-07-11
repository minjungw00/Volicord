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
    let mut output = format!(
        "Task lifecycle: {}\nVolicord record effect for this command: {}\nProfile: {}\nWrite Ticket: {}\nEvidence: {}\nPending user judgments: {}\nUnrecorded Product Repository changes: {}\nClose readiness: {}\nTransport: {}\n",
        summary_value_text(&card.task),
        authority_record_effect_text(&card.recording),
        summary_value_text(&card.profile),
        summary_value_text(&card.write_ticket),
        summary_value_text(&card.evidence),
        pending_user_judgments_text(&card.user_judgment),
        summary_value_text(&card.changes),
        summary_value_text(&card.close_status),
        summary_value_text(&card.transport),
    );
    append_summary_next(&mut output, &card.next);
    output.push_str(&does_not_prove_line(summary_card_non_guarantees(card)));
    output
}

fn summary_value_text(value: &str) -> &str {
    match value {
        "not_selected" => "not shown in this view",
        value => value,
    }
}

fn authority_record_effect_text(value: &str) -> String {
    let effect = match value {
        "read_only" => "none",
        "core_committed" => "recorded",
        "diagnostic_observation" => "local diagnostic observation only",
        "not_selected" => return "not shown in this view".to_owned(),
        value => return format!("status code `{value}`"),
    };
    format!("{effect} (does not describe product-file writes or Runtime Home write capability)")
}

fn pending_user_judgments_text(value: &str) -> &str {
    match value {
        "none" => "pending (0)",
        value => summary_value_text(value),
    }
}

fn append_summary_next(output: &mut String, next: &str) {
    let Some((label, command)) = backticked_volicord_command(next) else {
        output.push_str(&format!(
            "Primary next action: {}\n",
            summary_value_text(next)
        ));
        return;
    };
    output.push_str(&format!(
        "Primary next action: {label}\n  Run:\n    {command}\n"
    ));
}

fn backticked_volicord_command(next: &str) -> Option<(String, String)> {
    let start = next.find("`volicord ")?;
    let command_start = start + 1;
    let command_end = next[command_start..].find('`')? + command_start;
    let command = next[command_start..command_end].to_owned();
    let replacement = if command == "volicord inbox" {
        "the CLI inbox"
    } else {
        "the command below"
    };
    let label = next
        .replace(&format!("`{command}`"), replacement)
        .trim()
        .to_owned();
    Some((label, command))
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

pub(crate) fn render_close_and_next_action_totals_text(value: &Value) -> String {
    format!(
        "Close readiness blockers (total): {}\nTop-level next actions (total): {}\n",
        top_level_array_count_text(value, "close_blockers"),
        top_level_array_count_text(value, "next_actions"),
    )
}

fn top_level_array_count_text(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_array)
        .map(|values| values.len().to_string())
        .unwrap_or_else(|| "not shown in this view".to_owned())
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
    let watcher_scan = coverage
        .get("watcher_scan_summary")
        .and_then(watcher_scan_summary_text)
        .unwrap_or_else(|| "watcher_scan=unavailable".to_owned());

    Some(format!(
        "Coverage: profile={profile}; host_hook={host_hook}; session_watcher={session_watcher}; started={started}; last_snapshot={last_snapshot}; unresolved_unrecorded_changes={unresolved}\nCoverage watcher scan: {watcher_scan}\nCoverage does not guarantee: {non_guarantees}\n"
    ))
}

fn watcher_scan_summary_text(value: &Value) -> Option<String> {
    if !value.is_object() {
        return None;
    }
    let files_scanned = value.get("files_scanned").and_then(Value::as_u64)?;
    let files_skipped = value.get("files_skipped").and_then(Value::as_u64)?;
    let unreadable = value
        .get("unreadable_paths_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let degraded_reasons = value
        .get("degraded_reasons")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let degraded_reasons = if degraded_reasons.is_empty() {
        "none".to_owned()
    } else {
        degraded_reasons.join(",")
    };
    let skipped_paths = value
        .get("skipped_paths_sample")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let skipped_paths = if skipped_paths.is_empty() {
        "none".to_owned()
    } else {
        skipped_paths.join(",")
    };
    let monitoring_note = if value
        .get("not_full_filesystem_monitoring")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        "not_full_filesystem_monitoring=yes"
    } else {
        "not_full_filesystem_monitoring=unknown"
    };
    Some(format!(
        "files_scanned={files_scanned}; files_skipped={files_skipped}; unreadable_paths={unreadable}; degraded_reasons={degraded_reasons}; skipped_paths_sample={skipped_paths}; {monitoring_note}"
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
                "watcher_scan_summary": {
                    "files_scanned": 12,
                    "files_skipped": 3,
                    "unreadable_paths_count": 1,
                    "degraded_reasons": ["file_size_limit", "unreadable_path"],
                    "skipped_paths_sample": ["large.bin", "private"],
                    "not_full_filesystem_monitoring": true
                },
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
        assert!(text.contains("files_scanned=12"));
        assert!(text.contains("files_skipped=3"));
        assert!(text.contains("unreadable_paths=1"));
        assert!(text.contains("file_size_limit,unreadable_path"));
        assert!(text.contains("not_full_filesystem_monitoring=yes"));
        assert!(text.contains("actor identity proof"));
        assert!(text.contains("full filesystem monitoring"));
        assert!(text.contains("write prevention"));
    }

    #[test]
    fn summary_card_text_renders_embedded_volicord_command_as_standalone_line() {
        let card = SummaryCard {
            task: "selected".to_owned(),
            recording: "read_only".to_owned(),
            profile: "record".to_owned(),
            write_ticket: "none".to_owned(),
            evidence: "none".to_owned(),
            user_judgment: "pending (1)".to_owned(),
            changes: "none".to_owned(),
            close_status: "blocked".to_owned(),
            transport: "local CLI".to_owned(),
            next: "Use `volicord inbox` to list and answer pending user-owned judgments."
                .to_owned(),
            next_action: None,
            guarantee: USER_CHANNEL_SUMMARY_GUARANTEE.to_owned(),
        };

        let text = render_summary_card_text(&card);

        assert!(text.contains("Task lifecycle: selected"));
        assert!(text.contains(
            "Volicord record effect for this command: none (does not describe product-file writes or Runtime Home write capability)"
        ));
        assert!(text.contains("Pending user judgments: pending (1)"));
        assert!(text.contains("Unrecorded Product Repository changes: none"));
        assert!(text.contains("Close readiness: blocked"));
        assert!(text.contains(
            "Primary next action: Use the CLI inbox to list and answer pending user-owned judgments."
        ));
        assert!(text.contains("  Run:\n    volicord inbox\n"));
        assert!(!text.contains("Primary next action: Use `volicord inbox`"));
        assert!(!text.contains("Recording: read_only"));
    }

    #[test]
    fn summary_card_text_humanizes_unselected_values_and_zero_pending_count() {
        let card = SummaryCard {
            task: "not_selected".to_owned(),
            recording: "diagnostic_observation".to_owned(),
            profile: "not_selected".to_owned(),
            write_ticket: "not_selected".to_owned(),
            evidence: "not_selected".to_owned(),
            user_judgment: "none".to_owned(),
            changes: "not_selected".to_owned(),
            close_status: "not_selected".to_owned(),
            transport: "local CLI".to_owned(),
            next: "none".to_owned(),
            next_action: None,
            guarantee: DIAGNOSTIC_SUMMARY_GUARANTEE.to_owned(),
        };

        let text = render_summary_card_text(&card);

        assert!(text.contains("Task lifecycle: not shown in this view"));
        assert!(text.contains(
            "Volicord record effect for this command: local diagnostic observation only (does not describe product-file writes or Runtime Home write capability)"
        ));
        assert!(text.contains("Profile: not shown in this view"));
        assert!(text.contains("Pending user judgments: pending (0)"));
        assert!(text.contains("Close readiness: not shown in this view"));
        assert!(text.contains("Primary next action: none"));
        assert!(!text.contains("not_selected"));
    }

    #[test]
    fn close_and_next_action_totals_count_complete_top_level_arrays() {
        let value = json!({
            "close_blockers": [{"code": "ONE"}, {"code": "TWO"}],
            "next_actions": [{"label": "one"}, {"label": "two"}, {"label": "three"}]
        });

        assert_eq!(
            render_close_and_next_action_totals_text(&value),
            "Close readiness blockers (total): 2\nTop-level next actions (total): 3\n"
        );
    }
}
