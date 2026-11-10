//! Actual recorded history, independent of article availability and acceptance.
use super::*;
pub(super) fn render(
    html: &mut String,
    r: &ViewerRequest,
    answers: &ReaderAnswers<'_>,
    work: &UnderstandingWork,
) {
    let anchor = document_anchor(answers, "timeline");
    html.push_str(&format!("<section class=\"recorded-timeline reading-section\" id=\"{}\" tabindex=\"-1\" aria-label=\"{}\"><p class=\"source-note\">{}</p>",escape(&anchor),text(r.locale,"Recorded timeline","기록의 흐름"),text(r.locale,"Latest records first. These are recorded reports; opening this page runs no checks.","마지막 기록부터 읽습니다. 기록된 보고이며 이 페이지에서 검사를 새로 실행하지 않습니다.")));
    if work.reading.states.is_empty() {
        html.push_str(&format!(
            "<p>{}</p>",
            text(r.locale, "No Checkpoint recorded", "Checkpoint 기록 없음")
        ));
    } else {
        html.push_str("<ol class=\"work-timeline\">");
        for (position, state) in work.reading.states.iter().enumerate().rev() {
            let time = if let Some((instant, calendar)) = observation_time(state.observed_at) {
                format!("<time datetime=\"{instant}\" data-recorded-at=\"{}\" title=\"Unix microseconds: {}\">{calendar}</time>",state.observed_at.as_unix_micros(),state.observed_at.as_unix_micros())
            } else {
                format!(
                    "<span>{}</span>",
                    escape(&timestamp_label(state.observed_at, r.locale))
                )
            };
            let event = format!(
                "{}-{}-r{}",
                document_anchor(answers, "event"),
                state.checkpoint_id,
                state.checkpoint_revision
            );
            let resumed = state.work_state == volicord_context::WorkState::InProgress
                && position
                    .checked_sub(1)
                    .and_then(|p| work.reading.states.get(p))
                    .is_some_and(|prior| prior.work_state == volicord_context::WorkState::Paused);
            let label = if resumed {
                text(r.locale, "Resumed · in progress", "재개 · 진행 중")
            } else {
                work_state_label(state.work_state, r.locale)
            };
            html.push_str(&format!("<li id=\"{event}\" tabindex=\"-1\" data-checkpoint-id=\"{}\" data-checkpoint-revision=\"{}\"><span class=\"timeline-dot\" aria-hidden=\"true\"></span><div><div class=\"timeline-label\"><strong>{label}</strong>{time}</div>",state.checkpoint_id,state.checkpoint_revision));
            if state.checkpoint_revision > 1 {
                html.push_str(&format!(
                    "<p class=\"source-note\">{}: {}</p>",
                    text(r.locale, "Record revision", "기록 revision"),
                    state.checkpoint_revision
                ));
            }
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
            html.push_str("<div class=\"timeline-facts\">");
            if state.verification.is_empty() {
                html.push_str(&format!(
                    "<p>{}</p>",
                    text(
                        r.locale,
                        "No verification report recorded.",
                        "검증 보고가 기록되지 않았습니다."
                    )
                ));
            } else {
                html.push_str(&format!(
                    "<p>{}: {} · ",
                    text(r.locale, "Verification reports", "검증 보고"),
                    state.verification.len()
                ));
                let mut separator = "";
                for status in [
                    volicord_context::VerificationState::Failed,
                    volicord_context::VerificationState::Partial,
                    volicord_context::VerificationState::NotRun,
                    volicord_context::VerificationState::Passed,
                ] {
                    let count = state
                        .verification
                        .iter()
                        .filter(|f| f.state == status)
                        .count();
                    if count > 0 {
                        html.push_str(&format!(
                            "{separator}{}: {count}",
                            verification_state_label(status, r.locale)
                        ));
                        separator = " · ";
                    }
                }
                html.push_str("</p>");
            }
            // Failure routes stay visible even while every original is folded.
            for (index, check) in work
                .reading
                .verification_report
                .checks
                .iter()
                .enumerate()
                .filter(|(_, c)| {
                    c.original.checkpoint_id == state.checkpoint_id.to_string()
                        && c.original.checkpoint_revision == state.checkpoint_revision
                        && c.reported_state == volicord_projections::ReportedCheckState::Failed
                })
            {
                let historical =
                    verification::historical_check(check, &work.reading.verification_report);
                let target = if index < 64 {
                    verification::check_anchor(answers, check)
                } else {
                    document_anchor(answers, "full-verification")
                };
                html.push_str(&format!(
                    "<p class=\"timeline-failure\"><a href=\"#{target}\">{} · {} {}</a></p>",
                    if historical {
                        text(r.locale, "Historical failure", "과거 검증 실패")
                    } else {
                        text(
                            r.locale,
                            "Unresolved failed report",
                            "해결 여부가 확인되지 않은 실패 보고",
                        )
                    },
                    text(r.locale, "Inspect report", "보고 확인"),
                    check.original.verification_index + 1
                ));
            }
            html.push_str(&format!(
                "<p>{}: {} · {}: {}</p></div>",
                text(r.locale, "Review", "검토"),
                user_review_label(state.user_review.state, r.locale),
                text(r.locale, "Acceptance", "수용"),
                user_acceptance_label(state.user_acceptance.state, r.locale)
            ));
            if let Some(change) = change {
                html.push_str(&format!("<details class=\"timeline-original\" id=\"{event}-report\"><summary>{}</summary><blockquote class=\"timeline-report\" lang=\"\" data-statement-role=\"verified-canonical\">{}</blockquote><a href=\"#{event}\">{}</a></details>",text(r.locale,"Original progress report","진행 보고 원문"),escape(change),text(r.locale,"Return to event","기록으로 돌아가기")));
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
            for (index, fact) in state.verification.iter().enumerate() {
                html.push_str(&format!("<details class=\"timeline-original\" id=\"{event}-verification-{}\"><summary>{} {} · {}</summary>",index+1,text(r.locale,"Original verification report","검증 보고 원문"),index+1,verification_state_label(fact.state,r.locale)));
                if let Some(outcome) = &fact.outcome {
                    html.push_str(&format!(
                        "<blockquote lang=\"\">{}</blockquote>",
                        escape(outcome)
                    ));
                } else {
                    html.push_str(&format!(
                        "<p>{}</p>",
                        text(
                            r.locale,
                            "No outcome text recorded.",
                            "기록된 결과 문구가 없습니다."
                        )
                    ));
                }
                html.push_str(&format!(
                    "<a href=\"#{event}\">{}</a></details>",
                    text(r.locale, "Return to event", "기록으로 돌아가기")
                ));
            }
            html.push_str(&format!("<details class=\"timeline-basis\"><summary>{}</summary><p>{} @{} · {}</p></details></div></li>",text(r.locale,"Record identity","기록 식별자"),state.checkpoint_id,state.checkpoint_revision,text(r.locale,"Checkpoint recording time","Checkpoint 기록 시각")));
        }
        html.push_str("</ol>");
    }
    html.push_str("</section>");
}
