//! Actual recorded history, independent of article availability and acceptance.
use super::*;
pub(super) fn render(html: &mut String, r: &ViewerRequest, anchor: &str, work: &UnderstandingWork) {
    html.push_str(&format!("<section class=\"recorded-timeline reading-section\" id=\"{}\"><h2>{}</h2><p class=\"source-note\">{}</p>",escape(anchor),text(r.locale,"Recorded timeline","기록의 흐름"),text(r.locale,"Latest records first. These are recorded reports; opening this page runs no checks.","마지막 기록부터 읽습니다. 기록된 보고이며 이 페이지에서 검사를 새로 실행하지 않습니다.")));
    if work.reading.states.is_empty() {
        html.push_str(&format!(
            "<p>{}</p>",
            text(r.locale, "No Checkpoint recorded", "Checkpoint 기록 없음")
        ));
    } else {
        html.push_str("<ol class=\"work-timeline\">");
        for state in work.reading.states.iter().rev() {
            let (instant, calendar) = observation_time(state.observed_at);
            html.push_str(&format!("<li data-checkpoint-id=\"{}\" data-checkpoint-revision=\"{}\"><span class=\"timeline-dot\" aria-hidden=\"true\"></span><div><div class=\"timeline-label\"><strong>{}</strong><time datetime=\"{}\" data-recorded-at=\"{}\">{}</time></div>",state.checkpoint_id,state.checkpoint_revision,work_state_label(state.work_state,r.locale),instant,state.observed_at.as_unix_micros(),calendar));
            let change = work
                .reading
                .changes
                .iter()
                .find(|c| {
                    c.basis.record
                        == volicord_projections::ReadingRecord::Checkpoint(state.checkpoint_id)
                        && c.basis.revision == state.checkpoint_revision
                })
                .and_then(|c| c.original_text.as_deref())
                .filter(|s| !s.trim().is_empty());
            if let Some(change) = change {
                html.push_str(&format!("<p class=\"timeline-report\" lang=\"\" data-statement-role=\"verified-canonical\">{}</p>",escape(change)));
            } else {
                html.push_str(&format!(
                    "<p class=\"timeline-report missing\">{}</p>",
                    text(
                        r.locale,
                        "No progress description recorded.",
                        "기록된 진행 설명이 없습니다."
                    )
                ));
            }
            html.push_str("<div class=\"timeline-facts\">");
            for fact in &state.verification {
                html.push_str(&format!(
                    "<p>{}: {}{}</p>",
                    text(r.locale, "Recorded verification", "기록된 검증"),
                    verification_state_label(fact.state, r.locale),
                    fact.outcome
                        .as_deref()
                        .map(|s| format!(" · {}", escape(s)))
                        .unwrap_or_default()
                ));
            }
            html.push_str(&format!("<p>{}: {} · {}: {}</p></div><details class=\"timeline-basis\"><summary>{}</summary><p>{} @{} · {}</p></details></div></li>",text(r.locale,"Review","검토"),user_review_label(state.user_review.state,r.locale),text(r.locale,"Acceptance","수용"),user_acceptance_label(state.user_acceptance.state,r.locale),text(r.locale,"Record identity","기록 식별자"),state.checkpoint_id,state.checkpoint_revision,text(r.locale,"Checkpoint recording time","Checkpoint 기록 시각")));
        }
        html.push_str("</ol>");
    }
    html.push_str("</section>");
}
