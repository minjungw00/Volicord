use serde_json::Value;
use volicord_types::SummaryCard;

use crate::disclosure::{
    does_not_prove_line, AUTHORITY_RECORD_NON_GUARANTEE_TEXT,
    DIAGNOSTIC_OBSERVATION_NON_GUARANTEE_TEXT, USER_CHANNEL_NON_GUARANTEE_TEXT,
};

pub(crate) const DIAGNOSTIC_SUMMARY_GUARANTEE: &str =
    "Local diagnostic observation; not OS enforcement, write prevention, actor attribution proof, correctness proof, test sufficiency proof, or review completion.";

pub(crate) const USER_CHANNEL_SUMMARY_GUARANTEE: &str =
    "Local User Channel view; listing does not resolve a user action or prove close readiness.";

pub(crate) fn render_summary_card_text(card: &SummaryCard) -> String {
    let mut output = format!(
        "Task lifecycle: {}\nVolicord record effect for this command: {}\nProfile: {}\nWrite Ticket: {}\nEvidence: {}\nPending user actions: {}\nUnrecorded Product Repository changes: {}\nClose readiness: {}\nTransport: {}\n",
        summary_value_text(&card.task),
        authority_record_effect_text(&card.recording),
        summary_value_text(&card.profile),
        summary_value_text(&card.write_ticket),
        summary_value_text(&card.evidence),
        pending_user_actions_text(&card.user_action),
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

fn pending_user_actions_text(value: &str) -> &str {
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
        DIAGNOSTIC_SUMMARY_GUARANTEE => DIAGNOSTIC_OBSERVATION_NON_GUARANTEE_TEXT,
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
    fn summary_card_text_renders_embedded_volicord_command_as_standalone_line() {
        let card = SummaryCard {
            task: "selected".to_owned(),
            recording: "read_only".to_owned(),
            profile: "record".to_owned(),
            write_ticket: "none".to_owned(),
            evidence: "none".to_owned(),
            user_action: "pending (1)".to_owned(),
            changes: "none".to_owned(),
            close_status: "blocked".to_owned(),
            transport: "local CLI".to_owned(),
            next: "Use `volicord inbox` to list and resolve pending user actions.".to_owned(),
            next_action: None,
            guarantee: USER_CHANNEL_SUMMARY_GUARANTEE.to_owned(),
        };

        let text = render_summary_card_text(&card);

        assert!(text.contains("Task lifecycle: selected"));
        assert!(text.contains(
            "Volicord record effect for this command: none (does not describe product-file writes or Runtime Home write capability)"
        ));
        assert!(text.contains("Pending user actions: pending (1)"));
        assert!(text.contains("Unrecorded Product Repository changes: none"));
        assert!(text.contains("Close readiness: blocked"));
        assert!(text.contains(
            "Primary next action: Use the CLI inbox to list and resolve pending user actions."
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
            user_action: "none".to_owned(),
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
        assert!(text.contains("Pending user actions: pending (0)"));
        assert!(text.contains("Close readiness: not shown in this view"));
        assert!(text.contains("Primary next action: none"));
        assert!(!text.contains("not_selected"));
    }

    #[test]
    fn summary_card_text_preserves_every_evidence_gate_state() {
        for state in [
            "not_required",
            "optional_none",
            "required_missing",
            "partial",
            "sufficient",
            "stale",
            "blocked",
        ] {
            let card = SummaryCard {
                task: "selected".to_owned(),
                recording: "read_only".to_owned(),
                profile: "record".to_owned(),
                write_ticket: "none".to_owned(),
                evidence: state.to_owned(),
                user_action: "none".to_owned(),
                changes: "none".to_owned(),
                close_status: "blocked".to_owned(),
                transport: "local CLI".to_owned(),
                next: "none".to_owned(),
                next_action: None,
                guarantee: "Local authority record.".to_owned(),
            };

            assert!(render_summary_card_text(&card).contains(&format!("Evidence: {state}\n")));
        }
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
