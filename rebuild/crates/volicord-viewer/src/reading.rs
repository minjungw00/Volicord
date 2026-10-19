//! Purpose-oriented sections shared by live and whole-snapshot rendering.
use super::*;
use crate::{CodeScope, ViewerTool, ViewerView};

fn url(request: &ViewerRequest, view: &ViewerView) -> String {
    let mut fields = view.fields();
    fields.push(("locale", locale_key(request.locale).into()));
    fields.push(("language", request.requested_language.clone()));
    format!(
        "/?{}",
        fields
            .into_iter()
            .map(|(k, v)| format!("{k}={}", percent_encode(&v)))
            .collect::<Vec<_>>()
            .join("&")
    )
}
fn link(
    html: &mut String,
    request: &ViewerRequest,
    view: ViewerView,
    label: &str,
    fragment: Option<&str>,
) {
    let href = fragment
        .map(|id| format!("#{id}"))
        .unwrap_or_else(|| url(request, &view));
    html.push_str(&format!(
        "<a href=\"{}\">{}</a>",
        escape(&href),
        escape(label)
    ));
}
pub(super) fn navigation(html: &mut String, r: &ViewerRequest, snapshot: bool) {
    html.push_str("<nav aria-label=\"Viewer\">");
    for (view, label, id) in [
        (
            ViewerView::Overview,
            text(r.locale, "Overview", "개요"),
            "overview",
        ),
        (
            ViewerView::RepositoryExplanation,
            text(r.locale, "Repository", "저장소 설명"),
            "repository-explanation",
        ),
        (
            ViewerView::Work { work: None },
            text(r.locale, "Work", "작업"),
            "works",
        ),
    ] {
        let href = if snapshot {
            format!("#{id}")
        } else {
            url(r, &view)
        };
        html.push_str(&format!(
            "<a class=\"nav-link\" href=\"{}\"{}>{}</a>",
            escape(&href),
            if !snapshot && r.view.key() == view.key() {
                " aria-current=\"page\""
            } else {
                ""
            },
            label
        ));
    }
    html.push_str("<details class=\"tools-menu\"><summary>");
    html.push_str(text(r.locale, "Tools", "도구"));
    html.push_str("</summary><div class=\"tool-links\">");
    let href = if snapshot {
        "#decision-reading".into()
    } else {
        url(r, &ViewerView::Decisions { decision: None })
    };
    html.push_str(&format!(
        "<a href=\"{}\">{}</a>",
        escape(&href),
        text(r.locale, "Decision inspection", "결정 검사")
    ));
    for (tool, label, id) in [
        (
            ViewerTool::Status,
            text(r.locale, "Analysis & privacy", "분석 및 개인정보"),
            "analysis-tool",
        ),
        (
            ViewerTool::Documents,
            text(r.locale, "Documents", "문서"),
            "documents-tool",
        ),
        (
            ViewerTool::Memory,
            text(r.locale, "Memory · inspect & edit", "기억 · 검사 및 수정"),
            "memory-tool",
        ),
        (
            ViewerTool::Evidence,
            text(r.locale, "Evidence & history", "근거 및 이력"),
            "evidence-tool",
        ),
    ] {
        let href = if snapshot {
            format!("#{id}")
        } else {
            url(r, &ViewerView::Tools { tool })
        };
        html.push_str(&format!(
            "<a href=\"{}\"{}>{}</a>",
            escape(&href),
            if !snapshot && r.view == (ViewerView::Tools { tool }) {
                " aria-current=\"page\""
            } else {
                ""
            },
            label
        ));
    }
    html.push_str("</div></details></nav>");
}

pub(super) fn tool_heading(html: &mut String, r: &ViewerRequest, tool: ViewerTool) {
    let (title, description) = match tool {
        ViewerTool::Status => (text(r.locale, "Analysis & privacy", "분석 및 개인정보"), text(r.locale, "Understand coverage and freshness, then inspect local recovery and provider settings.", "분석 범위와 최신성을 확인하고 로컬 복구 안내와 제공자 설정을 살펴봅니다.")),
        ViewerTool::Documents => (text(r.locale, "Documents", "문서"), text(r.locale, "Read grounded previews and explicitly export the document you need.", "근거가 연결된 미리보기를 읽고 필요한 문서를 명시적으로 내보냅니다.")),
        ViewerTool::Memory => (text(r.locale, "Memory", "기억"), text(r.locale, "Inspect original records and retained Candidates. Corrections and forgetting require an explicit authorized action.", "원래 기록과 보존된 Candidate를 검사합니다. 수정과 삭제는 명시적으로 승인된 동작이 필요합니다.")),
        ViewerTool::Evidence => (text(r.locale, "Evidence & history", "근거 및 이력"), text(r.locale, "Inspect original observations, repository inventory and the full diagnostic record.", "원래 관측, 저장소 목록과 전체 진단 기록을 검사합니다.")),
    };
    html.push_str("<header class=\"page-head\"><p class=\"eyebrow\">PROJECT TOOLS</p>");
    heading(html, 1, title);
    html.push_str(&format!("<p class=\"intro\">{}</p></header>", description));
}

pub(super) fn runtime_blockers(
    html: &mut String,
    r: &ViewerRequest,
    health: &volicord_operations::HealthReport,
) {
    let issues = health
        .issues
        .iter()
        .filter(|i| i.scope == "canonical" || i.scope == "runtime")
        .collect::<Vec<_>>();
    if issues.is_empty() {
        return;
    }
    html.push_str("<aside class=\"runtime-blocker\" aria-label=\"");
    html.push_str(text(r.locale, "Runtime blocker", "런타임 차단 문제"));
    html.push_str("\"><h2>");
    html.push_str(text(
        r.locale,
        "Runtime needs attention",
        "런타임 확인 필요",
    ));
    html.push_str("</h2><ul>");
    for issue in issues {
        list_item(html, &format!("{}: {}", issue.scope, issue.detail));
    }
    html.push_str("</ul><p class=\"next-action\">");
    html.push_str(text(
        r.locale,
        "Inspect local health with volicord doctor; use its supported repair guidance.",
        "volicord doctor로 로컬 상태를 확인하고 제공되는 복구 안내를 따르세요.",
    ));
    html.push_str("</p></aside>");
}
pub(super) fn contextual_limits(
    html: &mut String,
    r: &ViewerRequest,
    p: &ProjectProjection,
    snapshot: bool,
) {
    let issues = p
        .answer_issues
        .iter()
        .filter(|i| i.kind != ProjectionIssueKind::Bound)
        .collect::<Vec<_>>();
    if issues.is_empty() && p.answer_capability_gaps.is_empty() {
        return;
    }
    html.push_str("<aside class=\"contextual-limits\"><h3>");
    html.push_str(text(r.locale, "Limits of this answer", "이 설명의 한계"));
    html.push_str("</h3><ul class=\"gap-list\">");
    for issue in issues {
        list_item(html, &format!("{}: {}", issue.affected_scope, issue.reason));
    }
    for gap in &p.answer_capability_gaps {
        list_item(
            html,
            &format!(
                "{} · {} · {} · {}: {}. {} {}",
                gap.area,
                capability_label(gap.capability, r.locale),
                capability_state_label(gap.state, r.locale),
                freshness_state_label(gap.freshness.state, r.locale),
                gap.reason,
                gap.user_visible_consequence.as_deref().unwrap_or(text(
                    r.locale,
                    "This scope cannot support a complete current code answer.",
                    "이 범위로 완전한 현재 코드 설명을 뒷받침할 수 없습니다."
                )),
                gap.usable_remainder.as_deref().unwrap_or(text(
                    r.locale,
                    "Canonical memory remains readable.",
                    "정식 기억은 계속 읽을 수 있습니다."
                ))
            ),
        );
    }
    html.push_str("</ul><p class=\"next-action\">");
    html.push_str(text(
        r.locale,
        "Inspect Analysis for affected coverage and local refresh or recovery guidance.",
        "분석에서 영향받은 범위와 로컬 갱신 또는 복구 안내를 확인하세요.",
    ));
    link(
        html,
        r,
        ViewerView::Tools {
            tool: ViewerTool::Status,
        },
        text(r.locale, "Open Analysis", "분석 열기"),
        snapshot.then_some("health"),
    );
    html.push_str("</p></aside>");
}
pub(super) fn diagnostics(
    html: &mut String,
    r: &ViewerRequest,
    p: &ProjectProjection,
    health: &volicord_operations::HealthReport,
) {
    html.push_str("<details id=\"diagnostics\" class=\"global-diagnostics\"><summary>");
    html.push_str(text(
        r.locale,
        "Repository and runtime diagnostics / exact omissions",
        "저장소 및 런타임 진단 / 정확한 생략",
    ));
    html.push_str("</summary><ul>");
    for issue in &health.issues {
        list_item(html, &format!("{}: {}", issue.scope, issue.detail));
    }
    for issue in &p.issues {
        list_item(
            html,
            &format!(
                "{}: {} · {}: {}",
                issue.affected_scope,
                issue.reason,
                text(r.locale, "Omitted", "생략"),
                issue.omitted_count
            ),
        );
    }
    for limit in &p.resume.known_limits {
        list_item(html, limit);
    }
    html.push_str("</ul>");
    for report in &p.repository_map.capabilities {
        html.push_str("<details><summary>");
        html.push_str(&escape(&format!(
            "{} · {}",
            capability_label(report.capability, r.locale),
            report.area.path
        )));
        html.push_str("</summary><pre>");
        html.push_str(&escape(&format!("{report:?}")));
        html.push_str("</pre></details>");
    }
    if r.requested_language != locale_key(r.locale) {
        empty_state(html,text(r.locale,"Interpretations require the exact requested language. Original quotations preserve their source language; fixed labels use the selected locale.","해석은 정확히 일치하는 요청 언어가 필요합니다. 원문 인용은 원래 언어, 고정 설명은 선택한 UI 언어를 사용합니다."));
    }
    html.push_str("</details>");
}
fn overview(
    html: &mut String,
    r: &ViewerRequest,
    p: &ProjectProjection,
    u: &ProjectUnderstanding,
    snapshot: bool,
) {
    html.push_str("<section id=\"overview\" class=\"snapshot-page\"><p class=\"eyebrow\">PROJECT OVERVIEW</p>");
    heading(html, 1, &p.overview.project_name);
    project_purpose(html, r, u);
    html.push_str(&format!("<div class=\"section-heading\"><div><h2>{}</h2></div><a class=\"button secondary\" href=\"{}\">{} →</a></div>",
        text(r.locale,"Recent Work","최근 작업"),
        if snapshot { "#works".into() } else { escape(&url(r,&ViewerView::Work {work:None})) },
        text(r.locale,"All Work","전체 작업 보기")));
    html.push_str(&format!("<p class=\"source-note\">{}</p><div class=\"work-list\">",text(r.locale,"Interpretations are based on Sources; verification, review and acceptance follow the original records.","Source에 근거한 해석 · 검증·검토·수락 상태는 원래 기록을 따릅니다.")));
    let mut recent = p
        .work_history
        .iter()
        .chain(&u.work_overview.current.items)
        .chain(&u.work_overview.completed.items)
        .chain(&u.work_overview.remaining.items)
        .collect::<Vec<_>>();
    recent.sort_by_key(|w| std::cmp::Reverse(w.observed_at));
    let mut seen = std::collections::BTreeSet::new();
    for w in recent
        .into_iter()
        .filter(|w| seen.insert(w.work_item_id))
        .take(3)
    {
        work_summary(html, r, w, snapshot);
    }
    if p.work_count == 0 {
        empty_state(
            html,
            text(
                r.locale,
                "No Work has been recorded.",
                "기록된 작업이 없습니다.",
            ),
        );
    }
    html.push_str("</div></section>");
}
fn project_purpose(html: &mut String, r: &ViewerRequest, u: &ProjectUnderstanding) {
    if u.project_purpose.is_empty() {
        html.push_str(&format!(
            "<p class=\"purpose-absence\">{}</p>",
            text(
                r.locale,
                "No Project purpose has been recorded.",
                "기록된 프로젝트 목적이 없습니다."
            )
        ));
        return;
    }
    html.push_str("<section class=\"project-goal\">");
    heading(html, 2, text(r.locale, "Project purpose", "프로젝트 목적"));
    {
        for purpose in &u.project_purpose {
            html.push_str(&format!("<p class=\"project-purpose\" data-statement-role=\"verified-canonical\" lang=\"\">{}</p>",escape(&purpose.statement)));
        }
        html.push_str(&format!(
            "<details><summary>{}</summary><pre>{}</pre></details>",
            text(r.locale, "Purpose basis", "목적의 근거"),
            escape(&format!("{:?}", u.project_purpose))
        ));
    }
    html.push_str("</section>");
}
fn work_summary(html: &mut String, r: &ViewerRequest, w: &UnderstandingWork, snapshot: bool) {
    let answers = volicord_projections::work_answers(w, &r.requested_language, r.locale.fixed());
    let (title, basis, language) = work_reading_heading(w, r);
    let href = if snapshot {
        format!("#work-{}", w.work_item_id)
    } else {
        url(
            r,
            &ViewerView::Work {
                work: Some(w.work_item_id),
            },
        )
    };
    html.push_str(&format!("<article class=\"work-card\" data-work-id=\"{}\" data-work-state=\"{}\"><div class=\"work-card-heading\"><h3 lang=\"{}\" data-title-basis=\"{}\">{}</h3><span class=\"work-status\">{}</span></div>",w.work_item_id,understanding_work_state_key(w.state),escape(&language),basis,escape(&title),understanding_work_state_label(w.state,r.locale)));
    if let Some(synopsis) = answers.synopsis() {
        html.push_str(&format!("<p class=\"work-summary\" data-question=\"Synopsis\" lang=\"{}\" data-content-language=\"{}\">{}</p>",escape(&content_language_tag(&r.requested_language)),escape(&r.requested_language),escape(&synopsis.text)));
        if answers.reading_title().is_none() {
            html.push_str(&format!(
                "<p class=\"content-unavailable\">{}</p>",
                text(
                    r.locale,
                    "Work name production is incomplete.",
                    "작업 이름 생산이 미완료입니다."
                )
            ));
        }
    } else {
        html.push_str(&format!("<p class=\"work-summary content-unavailable\">{}</p>",text(r.locale,
            "Name and summary production is incomplete. Read the original instructions and available records in Work detail.",
            "이름과 요약 생산이 미완료입니다. 작업 상세에서 원래 지시와 사용 가능한 기록을 읽을 수 있습니다.")));
    }
    html.push_str("<div class=\"work-card-footer\">");
    work_event_times(html, r, w);
    html.push_str(&format!(
        "<a class=\"button secondary card-action\" href=\"{}\">{} →</a></div></article>",
        escape(&href),
        text(r.locale, "Read Work", "작업 읽기")
    ));
}

fn work_event_times(html: &mut String, r: &ViewerRequest, w: &UnderstandingWork) {
    html.push_str("<div class=\"work-dates\">");
    for (label, event, absent) in [
        (
            text(r.locale, "Created", "생성"),
            w.creation_event.as_ref(),
            text(
                r.locale,
                "Original creation history unavailable",
                "최초 생성 이력 확인 불가",
            ),
        ),
        (
            text(r.locale, "Latest work record", "마지막 작업 기록"),
            w.latest_work_record_event.as_ref(),
            text(r.locale, "No Checkpoint recorded", "Checkpoint 기록 없음"),
        ),
    ] {
        html.push_str(&format!("<span class=\"work-date\"><span>{}</span>", label));
        if let Some(event) = event {
            let (instant, calendar) = observation_time(TimestampMicros::from_unix_micros(
                event.recorded_at_unix_micros,
            ));
            html.push_str(&format!("<time datetime=\"{}\" data-recorded-at=\"{}\" data-event-kind=\"{}\" data-record-kind=\"{}\" data-record-id=\"{}\" data-revision=\"{}\" title=\"{} @{} · {}\">{}</time>",instant,event.recorded_at_unix_micros,event.kind,event.record_kind,escape(&event.record_identity),event.revision,escape(&event.record_identity),event.revision,event.kind,calendar));
        } else {
            html.push_str(&format!("<span class=\"event-absence\">{}</span>", absent));
        }
        html.push_str("</span>");
    }
    html.push_str("</div>");
}

/// A permitted authored heading is derived wording, never canonical title
/// authority. Missing headings retain the full Goal and exact basis in detail.
pub(super) fn work_reading_heading(
    w: &UnderstandingWork,
    r: &ViewerRequest,
) -> (String, &'static str, String) {
    let answers = volicord_projections::work_answers(w, &r.requested_language, r.locale.fixed());
    if let Some(title) = answers.reading_title() {
        return (
            title.into(),
            "generated-interpretation",
            content_language_tag(&r.requested_language),
        );
    }
    (
        text(r.locale, "Recorded Work", "작업 기록").into(),
        "neutral-label",
        locale_key(r.locale).into(),
    )
}

fn answer_title<'a>(question: &str, locale: ViewerLocale) -> &'a str {
    match question {
        "SectionHeading" | "Narrative" | "FlowStep" => text(locale, "Article", "설명"),
        "ComparisonCondition" => text(locale, "Condition", "조건"),
        "ComparisonResult" => text(locale, "Result", "결과"),
        "Heading" => text(locale, "Work heading · interpretation", "작업 제목 · 해석"),
        "Synopsis" => text(locale, "Synopsis", "요약"),
        "Observation" => text(locale, "Observations", "관측과 범위"),
        "Comparison" => text(locale, "Conditions and outcomes", "조건별 관측"),
        "ReadingPoint" => text(locale, "Read the mechanism", "작동 원리"),
        "Purpose" => text(locale, "Problem", "문제와 요청"),
        "ReportedChange" => text(locale, "Change / investigation", "구조와 동작"),
        "ExpectedEffect" => text(locale, "Expected effect", "기대 효과"),
        "Verification" => text(locale, "Verification", "검증"),
        "NextStep" => text(
            locale,
            "Host interpretation: next steps / uncertainty",
            "호스트 해석: 다음 단계 / 불확실성",
        ),
        "UserRationale" => text(locale, "User rationale", "사용자 이유"),
        "Recommendation" => text(locale, "Agent recommendation basis", "에이전트 권고 근거"),
        "Consequences" => text(locale, "Alternatives / trade-offs", "대안 / 절충"),
        "Applicability" => text(locale, "Applicability", "적용 범위"),
        "Limits" => text(locale, "Remaining uncertainty", "결과를 읽을 때의 한계"),
        _ => text(locale, "Explanation availability", "설명 가용성"),
    }
}
/// Stable within the exact realization, including repeated roles/shared ranges.
fn reader_claim_anchor(answers: &volicord_projections::QuestionAnswers, index: usize) -> String {
    let identity = answers
        .provenance
        .as_ref()
        .map(|p| format!("{}-{}", p.subject.key(), p.realization_fingerprint))
        .unwrap_or_else(|| {
            volicord_projections::explanation_text_hash(&format!("{:?}", answers.facts))
        });
    format!("reader-{}-p{}", fragment_identity(&identity), index + 1)
}
fn source_point_anchor(
    answers: &volicord_projections::QuestionAnswers,
    unit: usize,
    citation: usize,
) -> String {
    format!("{}-c{}", reader_claim_anchor(answers, unit), citation + 1)
}
fn reader_citations(
    html: &mut String,
    r: &ViewerRequest,
    answers: &volicord_projections::QuestionAnswers,
    index: usize,
) {
    html.push_str("<div class=\"evidence-actions\">");
    for (n, c) in answers.prose[index].citations.iter().enumerate() {
        html.push_str(&format!("<a class=\"evidence-link\" data-origin=\"{}\" href=\"#{}\">{} · {} {} ↗<span>{}:{}–{}</span></a>",reader_claim_anchor(answers,index),source_point_anchor(answers,index,n),answer_title(&answers.prose[index].question,r.locale),text(r.locale,"Source","근거"),n+1,escape(&c.locator),c.start_line,c.end_line));
    }
    html.push_str("</div>");
}

fn document_anchor(answers: &volicord_projections::QuestionAnswers, section: &str) -> String {
    format!("{}-{section}", reader_claim_anchor(answers, 0))
}
fn reader_unit(
    html: &mut String,
    r: &ViewerRequest,
    answers: &volicord_projections::QuestionAnswers,
    index: usize,
) {
    let answer = &answers.prose[index];
    let outer = match answer.question.as_str() {
        "ComparisonCondition" => "dt",
        "ComparisonResult" => "dd",
        "FlowStep" => "li",
        _ => "div",
    };
    let tag = if answer.question == "SectionHeading" {
        "h2"
    } else {
        "p"
    };
    html.push_str(&format!("<{outer} class=\"unit reading-section\" id=\"{}\" tabindex=\"-1\"><{tag} class=\"unit-text\" data-question=\"{}\" lang=\"{}\" data-content-language=\"{}\">{}</{tag}>",reader_claim_anchor(answers,index),escape(&answer.question),escape(&content_language_tag(&r.requested_language)),escape(&r.requested_language),escape(&answer.text)));
    reader_citations(html, r, answers, index);
    html.push_str(&format!("</{outer}>"));
}
pub(super) fn render_answers(
    html: &mut String,
    r: &ViewerRequest,
    answers: &volicord_projections::QuestionAnswers,
    work: Option<&UnderstandingWork>,
    snapshot: bool,
) {
    render_document(html, r, answers, work, None, "", "", snapshot);
}
#[allow(clippy::too_many_arguments)]
fn render_document(
    html: &mut String,
    r: &ViewerRequest,
    answers: &volicord_projections::QuestionAnswers,
    work: Option<&UnderstandingWork>,
    decision: Option<&volicord_projections::UnderstandingDecision>,
    records: &str,
    related: &str,
    snapshot: bool,
) {
    let has_limits = answers.prose.iter().any(|a| a.question == "Limits")
        || answers.facts.iter().any(|a| {
            matches!(
                a.question.as_str(),
                "RecordedLimits" | "RecordedNonGoals" | "KnownLimit"
            )
        });
    let current = answers.provenance.is_some();
    let repository = work.is_none() && decision.is_none();
    let (back_view, back_id, back_label, kind) = if work.is_some() {
        (
            ViewerView::Work { work: None },
            "works",
            text(r.locale, "← Work list", "← 작업 목록"),
            "WORK DOCUMENT",
        )
    } else if decision.is_some() {
        (
            ViewerView::Decisions { decision: None },
            "decision-reading",
            text(r.locale, "← Decisions", "← 결정 목록"),
            "DECISION DOCUMENT",
        )
    } else {
        (
            ViewerView::Overview,
            "overview",
            text(r.locale, "← Overview", "← 개요"),
            "REPOSITORY DOCUMENT",
        )
    };
    html.push_str(&format!("<article class=\"doc\"{}><p class=\"breadcrumb\"><a href=\"{}\">{}</a></p><header class=\"doc-head\"><p class=\"eyebrow\">{} · {}</p>",
        work.map(|w|format!(" data-work-id=\"{}\" data-work-state=\"{}\"",w.work_item_id,understanding_work_state_key(w.state))).unwrap_or_default(),
        if snapshot { format!("#{back_id}") } else { escape(&url(r, &back_view)) }, back_label, kind, text(r.locale,"interpretation and observations","해석과 관측")));
    if let Some((index, title)) = answers
        .prose
        .iter()
        .enumerate()
        .find(|(_, a)| a.question == "Heading")
    {
        html.push_str(&format!("<div id=\"{}\" tabindex=\"-1\"><h1 data-title-basis=\"generated-interpretation\" data-question=\"Heading\" lang=\"{}\" data-content-language=\"{}\">{}</h1></div>",reader_claim_anchor(answers,index),escape(&content_language_tag(&r.requested_language)),escape(&r.requested_language),escape(&title.text)));
    } else {
        html.push_str(&format!(
            "<h1 data-title-basis=\"neutral-label\" lang=\"{}\">{}</h1>",
            locale_key(r.locale),
            if work.is_some() {
                text(r.locale, "Recorded Work", "작업 기록")
            } else if decision.is_some() {
                text(r.locale, "Recorded Decision", "결정 기록")
            } else {
                text(r.locale, "Repository explanation", "저장소 설명")
            }
        ));
    }

    for (index, lead) in answers
        .prose
        .iter()
        .enumerate()
        .filter(|(_, a)| a.question == "Synopsis")
    {
        html.push_str(&format!("<div id=\"{}\" tabindex=\"-1\"><p class=\"lead\" data-question=\"Synopsis\" lang=\"{}\" data-content-language=\"{}\">{}</p></div>",reader_claim_anchor(answers,index),escape(&content_language_tag(&r.requested_language)),escape(&r.requested_language),escape(&lead.text)));
    }
    if let Some(w) = work {
        html.push_str(&format!(
            "<p class=\"record-strip\"><span>{}: {}</span></p>",
            text(r.locale, "Recorded state", "기록된 상태"),
            understanding_work_state_label(w.state, r.locale)
        ));
        work_event_times(html, r, w);
    }
    if current {
        html.push_str(&format!("<p class=\"interpretation\">{}</p>",text(r.locale,"Source-grounded host interpretation · verification, review and acceptance follow the original records.","Source에 근거한 호스트 해석 · 검증·검토·수락 상태는 원래 기록을 따릅니다.")));
    }
    if let Some(d) = decision {
        html.push_str(&format!("<p class=\"lead\" data-statement-role=\"verified-canonical\">{}</p><p class=\"record-strip\">{}</p>", escape(&decision_choice_attribution(&d.decision, r.locale)), brief_decision_state_label(d.decision.state, r.locale)));
    }
    html.push_str("</header>");
    opening_context(html, r, answers, work, decision);
    explanation_diagnostic(html, r, answers);
    if !current {
        let quotes = if decision.is_some() {
            &["QuestionContext", "UserRationale"][..]
        } else {
            &["ReportedResult"][..]
        };
        for fact in answers.facts.iter().filter(|a| {
            quotes.contains(&a.question.as_str())
                && (a.question != "ReportedResult"
                    || work.is_some_and(|w| w.reading.answers.result.is_some()))
        }) {
            html.push_str(&format!("<section class=\"reading-section recorded-reading\"><p class=\"unit-text\" data-question=\"{}\" lang=\"\" data-statement-role=\"verified-canonical\">{}</p></section>", escape(&fact.question), escape(&fact.text)));
        }
        if let Some(d) = decision {
            if let Some(rationale) = &d.reading.user_rationale.original_text {
                html.push_str(&format!("<section class=\"reading-section\"><h2>{}</h2><blockquote lang=\"\">{}</blockquote></section>", text(r.locale, "Recorded user rationale · quotation", "기록된 사용자 이유 · 인용"), escape(rationale)));
            }
        }
    }
    html.push_str("<nav class=\"audit-links\" aria-label=\"Document sections\">");
    for (section, label) in [
        (
            "reading",
            text(r.locale, "Read explanation ↓", "설명 읽기 ↓"),
        ),
        (
            "limits",
            text(
                r.locale,
                "Result / verification limits ↓",
                "결과·검증 한계 ↓",
            ),
        ),
        (
            "original",
            text(r.locale, "Original instructions", "원래 지시"),
        ),
        (
            "verification",
            text(r.locale, "Verification report", "검증 보고"),
        ),
        (
            "evidence-index",
            text(r.locale, "Evidence index", "근거 색인"),
        ),
        ("audit", text(r.locale, "Full evidence", "전체 근거")),
    ] {
        if ((section == "original" || section == "verification") && work.is_none())
            || (section == "limits" && !has_limits)
        {
            continue;
        }
        html.push_str(&format!(
            "<a class=\"button {}\" href=\"#{}\">{}</a>",
            if section == "reading" {
                "primary"
            } else {
                "secondary"
            },
            document_anchor(answers, section),
            label
        ));
    }
    html.push_str(&format!(
        "</nav><div id=\"{}\" tabindex=\"-1\">",
        document_anchor(answers, "reading")
    ));
    let mut flow_open = false;
    for (index, answer) in answers
        .prose
        .iter()
        .enumerate()
        .filter(|(_, a)| !matches!(a.question.as_str(), "Heading" | "Synopsis" | "Limits"))
    {
        if answer.question != "FlowStep" && flow_open {
            html.push_str("</ol>");
            flow_open = false;
        }
        if answer.question == "FlowStep" && !flow_open {
            html.push_str("<ol class=\"article-flow\">");
            flow_open = true;
        }
        if answer.question == "ComparisonCondition" {
            html.push_str("<dl class=\"comparison-card\">");
        }
        if answer.question == "Comparison" {
            html.push_str("<div class=\"comparisons\"><div class=\"comparison-card\">");
        }
        reader_unit(html, r, answers, index);
        if answer.question == "Comparison" {
            html.push_str("</div></div>");
        }
        if answer.question == "ComparisonResult" {
            html.push_str("</dl>");
        }
    }
    if flow_open {
        html.push_str("</ol>");
    }
    html.push_str("</div>");
    if let Some(work) = work {
        render_verification(html, r, answers, &work.reading.verification_report);
    }
    render_evidence_index(html, r, answers);
    if has_limits {
        html.push_str(&format!("<section class=\"reading-section limits-section\" id=\"{}\" tabindex=\"-1\"><aside class=\"guard\"><h2>{}</h2>",document_anchor(answers,"limits"),answer_title("Limits",r.locale)));
        for (index, _) in answers
            .prose
            .iter()
            .enumerate()
            .filter(|(_, a)| a.question == "Limits")
        {
            reader_unit(html, r, answers, index);
        }
        for fact in answers.facts.iter().filter(|a| {
            matches!(
                a.question.as_str(),
                "RecordedLimits" | "RecordedNonGoals" | "KnownLimit"
            )
        }) {
            html.push_str(&format!(
                "<p class=\"unit-text\" data-question=\"{}\" lang=\"\">{}</p>",
                escape(&fact.question),
                escape(&fact.text)
            ));
        }
        html.push_str("</aside></section>");
    }
    if !related.is_empty() {
        html.push_str("<section class=\"reading-section related-decisions\">");
        heading(
            html,
            2,
            if decision.is_some() {
                text(r.locale, "Related Work", "관련 작업")
            } else {
                text(r.locale, "Related Decisions", "관련 결정")
            },
        );
        html.push_str(related);
        html.push_str("</section>");
    }
    html.push_str("<div class=\"audit\">");
    if let Some(w) = work {
        html.push_str(&format!("<details class=\"record-box\" id=\"{}\"{}><summary>{}</summary><blockquote class=\"goal\" lang=\"\" data-statement-role=\"verified-canonical\">{}</blockquote><pre>{}</pre></details>",document_anchor(answers,"original"),if snapshot {" open"}else{""},text(r.locale,"Original instructions · complete Goal","원래 지시 · 전체 Goal"),escape(w.reading.goal.original_text.as_deref().unwrap_or(&w.title)),escape(&format!("{:?}",w.reading.goal.basis))));
    }
    html.push_str(&format!("<details class=\"record-box\" id=\"{}\"{}><summary>{}</summary><div class=\"record-facts\" data-statement-role=\"deterministic-derived\">",document_anchor(answers,"audit"),if snapshot{" open"}else{""},text(r.locale,"Full evidence · states · output identity","전체 근거 · 상태 · 출력 identity")));
    for fact in &answers.facts {
        html.push_str(&format!(
            "<p data-question=\"{}\">{}</p>",
            escape(&fact.question),
            escape(&fact.text)
        ));
    }
    html.push_str("</div>");
    if let Some(provenance) = &answers.provenance {
        repository_boundaries(html, r, answers, provenance);
        html.push_str(&format!(
            "<p>{}</p><pre>{}</pre>",
            text(
                r.locale,
                "Host interpretation; semantic accuracy has not been independently established.",
                "호스트 해석입니다. 의미 정확성은 독립적으로 확인되지 않았습니다."
            ),
            escape(&format!("{provenance:?}"))
        ));
        for (index, _) in answers
            .prose
            .iter()
            .enumerate()
            .filter(|(_, a)| matches!(a.question.as_str(), "Heading" | "Synopsis"))
        {
            reader_citations(html, r, answers, index);
        }
    }
    if let Some(w) = work {
        for state in &w.reading.states {
            html.push_str(&format!(
                "<section class=\"state-observation\"><h3>{} · {}</h3>",
                timestamp_label(state.observed_at, r.locale),
                work_state_label(state.work_state, r.locale)
            ));
            for fact in &state.verification {
                html.push_str(&format!(
                    "<p>{}: {} · {}</p>",
                    text(r.locale, "Verification", "검증"),
                    verification_state_label(fact.state, r.locale),
                    escape(fact.outcome.as_deref().unwrap_or(""))
                ));
            }
            html.push_str(&format!(
                "<p>{}: {} · {}: {}</p><pre>{}</pre></section>",
                text(r.locale, "Review", "검토"),
                user_review_label(state.user_review.state, r.locale),
                text(r.locale, "Acceptance", "수용"),
                user_acceptance_label(state.user_acceptance.state, r.locale),
                escape(&format!("{state:?}"))
            ));
        }
        html.push_str(&format!("<pre>{}</pre>", escape(&format!("Work: {}\nGoal basis: {:?}\nchanges: {:?}\nCheckpoints: {:?}\nDecisions: {:?}\nQuestions: {:?}\nresult: {:?}\nnext step: {:?}", w.work_item_id,w.reading.goal.basis,w.reading.changes,w.checkpoint_ids,w.decision_ids,w.open_question_ids,w.reading.answers.result,w.reading.next_step))));
    }
    if repository {
        html.push_str("<p>");
        link(
            html,
            r,
            ViewerView::Code {
                scope: CodeScope::Repository,
                entity: None,
            },
            text(r.locale, "Inspect repository structure", "저장소 구조 확인"),
            snapshot.then_some("code"),
        );
        html.push_str("</p>");
    }
    html.push_str(records);
    html.push_str("</details></div></article>");
    render_source_excerpts(html, r, answers, snapshot);
}

fn render_evidence_index(
    html: &mut String,
    r: &ViewerRequest,
    answers: &volicord_projections::QuestionAnswers,
) {
    html.push_str(&format!(
        "<section class=\"reading-section\" id=\"{}\" tabindex=\"-1\"><h2>{}</h2>",
        document_anchor(answers, "evidence-index"),
        text(r.locale, "Evidence index", "근거 색인")
    ));
    for entry in &answers.evidence_index {
        let Some(unit) = answers.prose.get(entry.unit_index) else {
            continue;
        };
        let preview: String = unit.text.chars().take(120).collect();
        let omitted = unit.text.len().saturating_sub(preview.len());
        html.push_str(&format!(
            "<details class=\"record-box\"><summary>{}</summary><a href=\"#{}\">{}</a>",
            escape(&preview),
            reader_claim_anchor(answers, entry.unit_index),
            text(r.locale, "Return to exact unit", "해당 문장으로 돌아가기")
        ));
        if omitted > 0 {
            html.push_str(&format!(
                "<p>{}: {omitted} UTF-8 bytes</p>",
                text(r.locale, "Index quotation omitted", "색인 인용 생략")
            ));
        }
        if let Some(provenance) = &answers.provenance {
            for evidence in provenance
                .evidence
                .iter()
                .filter(|e| entry.evidence_keys.contains(&e.key))
            {
                html.push_str(&format!(
                    "<p>{} · {} {}@{} · {}</p>",
                    escape(&evidence.key),
                    escape(&evidence.record_kind),
                    escape(&evidence.identity),
                    evidence.revision,
                    escape(&evidence.field)
                ));
            }
        }
        reader_citations(html, r, answers, entry.unit_index);
        html.push_str("</details>");
    }
    html.push_str("</section>");
}
fn render_verification(
    html: &mut String,
    r: &ViewerRequest,
    answers: &volicord_projections::QuestionAnswers,
    report: &volicord_projections::WorkVerificationReport,
) {
    use volicord_projections::EvidenceDetail;
    html.push_str(&format!("<section class=\"reading-section verification-report\" id=\"{}\" tabindex=\"-1\"><h2>{}</h2><p>{}</p>",document_anchor(answers,"verification"),text(r.locale,"Verification report","검증 보고"),text(r.locale,"Checkpoint reports and observed executions are separate. Exit status proves no assertions. Unrecorded test IDs, environment, coverage and comparable retest stay unknown.","Checkpoint 보고와 실행 관측은 별개입니다. 종료 코드만으로 assertion을 확정하지 않습니다. 미기록 테스트 ID·환경·coverage·비교 가능한 재검증은 미관측입니다.")));
    for check in report.checks.iter().take(64) {
        html.push_str(&format!("<article class=\"verification-check\"><h3>{}</h3><dl><dt>{}</dt><dd>{}</dd><dt>Checkpoint</dt><dd>{}@{} verification[{}]</dd><dt>Source</dt><dd>{}</dd><dt>{}</dt><dd>{:?}</dd></dl>",check.reported_state.label(r.locale.fixed()),text(r.locale,"Reported at","보고 시각"),timestamp_label(TimestampMicros::from_unix_micros(check.reported_at_unix_micros),r.locale),check.original.checkpoint_id,check.original.checkpoint_revision,check.original.verification_index,escape(check.source_id.as_deref().unwrap_or("—")),text(r.locale,"Applicability","적용성"),check.applicability));
        if let Some(quote) = &check.original_outcome {
            html.push_str(&format!(
                "<details><summary>{}</summary><blockquote lang=\"\">{}</blockquote></details>",
                text(
                    r.locale,
                    "Original verification report quotation",
                    "원래 검증 보고 인용"
                ),
                escape(quote)
            ));
        }
        html.push_str(&format!("<p>{}</p></article>",text(r.locale,"Comparison unresolved: no explicit comparable check/run relationship is retained. Later independent success does not repair this observation.","비교 미해결: 비교 가능한 검증·실행 관계가 명시적으로 보존되지 않았습니다. 나중의 독립 성공은 이 관측을 복구하지 않습니다.")));
    }
    for execution in report.executions.iter().take(32) {
        html.push_str(&format!("<article class=\"verification-execution\"><h3>{}</h3><dl><dt>Source</dt><dd>{}@{}</dd><dt>{}</dt><dd>{}</dd><dt>Exit / termination</dt><dd>{:?} / {:?}</dd><dt>{}</dt><dd>{:?}</dd></dl>",escape(&execution.command_label),execution.source_id,execution.source_revision,text(r.locale,"Recorded at","기록 시각"),timestamp_label(TimestampMicros::from_unix_micros(execution.recorded_at_unix_micros),r.locale),execution.exit_code,execution.termination,text(r.locale,"Retained observation","보존 관측"),execution.body_state));
        if let EvidenceDetail::Observed(o) = &execution.observation {
            html.push_str(&format!("<dl><dt>{}</dt><dd>{} → {}</dd><dt>Operation</dt><dd>{}</dd><dt>Analysis / Repository Snapshot</dt><dd>{} / {}</dd><dt>{}</dt><dd>{}</dd><dt>stdout / stderr</dt><dd>{:?} ({} bytes) / {:?} ({} bytes)</dd></dl>",text(r.locale,"Execution window","실행 시각"),timestamp_label(TimestampMicros::from_unix_micros(o.started_at_unix_micros),r.locale),timestamp_label(TimestampMicros::from_unix_micros(o.ended_at_unix_micros),r.locale),escape(&o.operation_id),escape(&o.analysis_snapshot_id),escape(&o.repository_snapshot_id),text(r.locale,"Target limitation","대상 한계"),text(r.locale,"Selected analysis only; execution-time tree was not rechecked","선택된 분석이며 실행 시점의 트리는 재확인하지 않았습니다"),o.stdout.representation,o.stdout.observed_bytes,o.stderr.representation,o.stderr.observed_bytes));
        }
        html.push_str("</article>");
    }
    html.push_str(&format!(
        "<p>{}: {} / {}. {}: {} / {}.</p></section>",
        text(r.locale, "Checks / executions", "보고 / 실행"),
        report.checks.len(),
        report.executions.len(),
        text(
            r.locale,
            "Displayed; full originals in evidence disclosure and CLI/MCP report",
            "표시 수; 전체 원본은 근거 상세와 CLI/MCP 보고"
        ),
        report.checks.len().min(64),
        report.executions.len().min(32)
    ));
}

fn repository_boundaries(
    html: &mut String,
    r: &ViewerRequest,
    answers: &volicord_projections::QuestionAnswers,
    provenance: &volicord_projections::ExplanationProvenance,
) {
    if !matches!(
        provenance.subject,
        volicord_projections::ExplanationSubject::Repository(_)
    ) {
        return;
    }
    let Some(basis) = provenance
        .evidence
        .iter()
        .find(|e| e.key == "repository_basis")
    else {
        return;
    };
    html.push_str(&format!(
        "<details id=\"{}\"><summary>{}</summary>",
        document_anchor(answers, "scope"),
        text(
            r.locale,
            "Included, excluded and unexamined boundaries",
            "포함·제외·미조사 경계"
        )
    ));
    for (key, label) in [
        (
            "included_areas",
            text(
                r.locale,
                "Included inventory (not all explained)",
                "포함된 목록 (모든 항목을 설명한 것은 아님)",
            ),
        ),
        (
            "excluded_areas",
            text(r.locale, "Excluded / unexamined", "제외 / 미조사"),
        ),
        (
            "unavailable_areas",
            text(r.locale, "Unavailable / unexamined", "이용 불가 / 미조사"),
        ),
    ] {
        heading(html, 3, label);
        html.push_str("<ul>");
        if let Some(areas) = basis.content["repository"][key].as_array() {
            for area in areas {
                if let Some(path) = area["path"].as_str() {
                    list_item(html, path);
                }
            }
            if areas.is_empty() {
                list_item(
                    html,
                    text(r.locale, "None recorded.", "기록된 항목이 없습니다."),
                );
            }
        }
        html.push_str("</ul>");
    }
    html.push_str("</details>");
}

fn opening_context(
    html: &mut String,
    r: &ViewerRequest,
    answers: &volicord_projections::QuestionAnswers,
    work: Option<&UnderstandingWork>,
    decision: Option<&volicord_projections::UnderstandingDecision>,
) {
    for fact in answers.facts.iter().filter(|a| {
        matches!(
            a.question.as_str(),
            "RepositoryScope" | "RepositoryCoverage"
        )
    }) {
        html.push_str(&format!(
            "<p class=\"scope-summary\" data-question=\"{}\">{}</p>",
            escape(&fact.question),
            escape(&fact.text)
        ));
    }
    if answers
        .facts
        .iter()
        .any(|a| a.question == "RepositoryScope")
    {
        html.push_str(&format!(
            "<p class=\"limit-shortcut\"><a href=\"#{}\">{}</a></p>",
            document_anchor(answers, "scope"),
            text(
                r.locale,
                "Inspect included, excluded and unexamined boundaries ↓",
                "포함·제외·미조사 경계 확인 ↓"
            )
        ));
    }
    let failed = work
        .and_then(|w| w.reading.answers.verification.as_ref())
        .is_some_and(|s| {
            s.verification
                .iter()
                .any(|v| v.state == VerificationState::Failed)
        });
    let rejected = work
        .and_then(|w| w.reading.answers.acceptance.as_ref())
        .is_some_and(|s| s.user_acceptance.state == UserAcceptanceState::Rejected);
    let important = answers.facts.iter().filter(|a| {
        matches!(
            a.question.as_str(),
            "HistoricalAdversity"
                | "VerificationCoverage"
                | "SourceEvidenceGap"
                | "Conflicts"
                | "ReviewBasis"
        ) || (rejected && a.question == "UserAcceptance")
    });
    let mut notice = String::new();
    if failed {
        notice.push_str(&format!("<p data-question=\"VerificationState\">{}</p>",text(r.locale,"Checkpoint verification includes failures. Completion does not establish passing verification.","Checkpoint 검증에 실패한 항목이 있습니다. 완료 상태가 검증 통과를 뜻하지는 않습니다.")));
    }
    for fact in important {
        notice.push_str(&format!(
            "<p data-question=\"{}\">{}</p>",
            escape(&fact.question),
            escape(&fact.text)
        ));
    }
    if !notice.is_empty() {
        html.push_str("<aside class=\"opening-limits\">");
        html.push_str(&notice);
        html.push_str(&format!(
            "<a href=\"#{}\">{}</a></aside>",
            document_anchor(answers, "audit"),
            text(
                r.locale,
                "Inspect exact states and observations ↓",
                "정확한 상태와 관측 확인 ↓"
            )
        ));
    }
    if decision.is_some() {
        for fact in answers
            .facts
            .iter()
            .filter(|a| a.question == "DeclaredScope")
        {
            html.push_str(&format!(
                "<p class=\"scope-summary\">{}</p>",
                escape(&fact.text)
            ));
        }
    }
    if answers.prose.iter().any(|a| a.question == "Limits")
        || answers.facts.iter().any(|a| {
            matches!(
                a.question.as_str(),
                "RecordedLimits" | "RecordedNonGoals" | "KnownLimit"
            )
        })
    {
        html.push_str(&format!(
            "<p class=\"limit-shortcut\"><a href=\"#{}\">{}</a></p>",
            document_anchor(answers, "limits"),
            text(
                r.locale,
                "Read the explanation's stated limits ↓",
                "설명에 명시된 한계 읽기 ↓"
            )
        ));
    }
}

fn explanation_diagnostic(
    html: &mut String,
    r: &ViewerRequest,
    answers: &volicord_projections::QuestionAnswers,
) {
    if let Some(diagnostic) = &answers.diagnostic {
        // Diagnostics remain exact in detail. The ordinary notice names only a
        // reason explicitly supplied by the dependency, never infers a cause.
        let reason = if diagnostic.contains("source_body_expired") {
            text(r.locale, "The retained Source body has expired; the interpretation cannot be shown as current.", "보존된 Source 본문의 보존 기간이 끝났습니다. 이 해석을 현재 유효한 설명으로 표시할 수 없습니다.")
        } else if diagnostic.contains("Source bytes changed") {
            text(r.locale, "Observed file bytes have changed; the interpretation cannot be shown as current.", "관측한 파일의 바이트가 변경되었습니다. 이 해석을 현재 유효한 설명으로 표시할 수 없습니다.")
        } else if diagnostic.contains("current Source unavailable")
            || diagnostic.contains("current excerpt unavailable")
        {
            text(r.locale, "A cited Source is no longer readable; the interpretation cannot be shown as current.", "인용된 Source를 더 이상 읽을 수 없습니다. 이 해석을 현재 유효한 설명으로 표시할 수 없습니다.")
        } else {
            text(r.locale, "The interpretation has an unavailable dependency or invalid content. Recorded facts remain readable; inspect the diagnostic for its exact cause.", "설명의 의존 근거를 이용할 수 없거나 내용이 유효하지 않습니다. 기록된 사실은 계속 읽을 수 있습니다. 정확한 원인은 진단에서 확인하세요.")
        };
        html.push_str(&format!(
            "<div class=\"contextual-limits explanation-diagnostic\"><p class=\"dependency-reason\"><strong>{}:</strong> {}</p><details><summary>{}</summary><p>{}</p></details></div>",
            text(r.locale, "Explanation limitation", "설명 한계"), reason,
            text(r.locale, "Exact dependency diagnostic", "정확한 의존 근거 진단"), escape(diagnostic)
        ));
    }
}

fn source_excerpt_anchor(
    answers: &volicord_projections::QuestionAnswers,
    c: &volicord_projections::ExplanationCitation,
) -> String {
    format!(
        "explanation-source-{}-{}-{}-{}-{}-{:?}",
        answers
            .provenance
            .as_ref()
            .map(|p| fragment_identity(&format!(
                "{}-{}",
                p.subject.key(),
                p.realization_fingerprint
            )))
            .unwrap_or_default(),
        c.source_id,
        c.field,
        c.start_byte,
        c.end_byte,
        c.use_kind
    )
}

fn citation_use_label(
    locale: ViewerLocale,
    use_kind: volicord_projections::ExplanationSourceUse,
) -> &'static str {
    match use_kind {
        volicord_projections::ExplanationSourceUse::CurrentCode => text(
            locale,
            "Observed file in selected snapshot",
            "선택 스냅샷에서 관측한 파일",
        ),
        volicord_projections::ExplanationSourceUse::BeforeCode => {
            text(locale, "Before: historical observation", "이전: 과거 관측")
        }
        volicord_projections::ExplanationSourceUse::ObservedExecution => {
            text(locale, "Observed command output", "관측된 명령 출력")
        }
    }
}

fn render_source_excerpts(
    html: &mut String,
    r: &ViewerRequest,
    answers: &volicord_projections::QuestionAnswers,
    snapshot: bool,
) {
    let Some(provenance) = &answers.provenance else {
        return;
    };
    if provenance.citations.is_empty() {
        return;
    }
    html.push_str("<aside class=\"appendix\" data-reading-role=\"cited-source\" data-statement-role=\"observed-fact\">");
    heading(
        html,
        2,
        text(r.locale, "Sources for this document", "이 문서의 Source"),
    );
    html.push_str(&format!(
        "<p class=\"source-note\">{}: {} / 16,384 UTF-8 bytes · 4,096 / {}</p>",
        text(
            r.locale,
            "Included unique cited text",
            "포함된 고유 인용 텍스트"
        ),
        answers
            .source_excerpts
            .iter()
            .map(|e| e.text.len())
            .sum::<usize>(),
        text(r.locale, "range maximum", "범위별 최대")
    ));
    let mut seen = std::collections::BTreeSet::new();
    for (index, answer) in answers.prose.iter().enumerate() {
        for (n, c) in answer.citations.iter().enumerate() {
            let anchor = source_point_anchor(answers, index, n);
            let excerpt = answers.source_excerpts.iter().find(|e| {
                source_excerpt_anchor(answers, &e.citation) == source_excerpt_anchor(answers, c)
            });
            let claim = answer
                .text
                .get(c.claim_start_byte..c.claim_end_byte)
                .unwrap_or("");
            html.push_str(&format!("<section class=\"source-point\" id=\"{}\" data-origin=\"{}\" data-source-id=\"{}\" tabindex=\"-1\"><div class=\"point-bar\"><a class=\"return\" href=\"#{}\">← {}</a><span class=\"point-counter\">{} · {} {}/{}</span></div><p class=\"point-title\">{}</p><blockquote class=\"claim-quote\" tabindex=\"0\" lang=\"{}\">{}</blockquote><h3 class=\"source-path\">{}:{}–{}</h3>",anchor,reader_claim_anchor(answers,index),escape(&c.source_id),reader_claim_anchor(answers,index),text(r.locale,"Return to explanation","설명으로 돌아가기"),answer_title(&answer.question,r.locale),text(r.locale,"Source","근거"),n+1,answer.citations.len(),text(r.locale,"Evidence for this explanation","이 설명을 확인하는 근거"),escape(&content_language_tag(&provenance.language)),escape(claim),escape(&c.locator),c.start_line,c.end_line));
            html.push_str(&format!("<div data-statement-role=\"generated-interpretation\" data-question=\"{}\" data-paragraph=\"{}\" data-claim-start=\"{}\" data-claim-end=\"{}\"></div>",escape(&answer.question),index+1,c.claim_start_byte,c.claim_end_byte));
            let range = source_excerpt_anchor(answers, c);
            if seen.insert(range.clone()) {
                html.push_str(&format!("<span id=\"{}\"></span>", escape(&range)));
            }
            html.push_str(&format!(
                "<p class=\"source-note\">{}</p>",
                citation_use_label(r.locale, c.use_kind)
            ));
            if let Some(e) = excerpt {
                if c.use_kind == volicord_projections::ExplanationSourceUse::CurrentCode {
                    let comparison=match e.comparison.as_deref() {
                        Some("changed")=>text(r.locale,"The explicit before/after file observations differ. Selected ranges are not a diff.","명시적 이전/이후 파일 관측이 다릅니다. 선택된 범위는 diff가 아닙니다."),
                        Some("no_change")=>text(r.locale,"The paired file observations have identical bytes. No change is shown.","짝지어진 파일 관측의 바이트가 동일합니다. 변경이 표시되지 않습니다."),
                        _=>text(r.locale,"Before-state was not retained. A before/after comparison is unavailable.","이전 상태가 보존되지 않았습니다. 이전/이후 비교를 이용할 수 없습니다."),
                    };
                    html.push_str(&format!("<p class=\"source-note\">{}</p>", comparison));
                    if let Some(before_index) = e.before_source_id.as_ref().and_then(|id| {
                        answer.citations.iter().position(|b| {
                            &b.source_id == id
                                && b.use_kind
                                    == volicord_projections::ExplanationSourceUse::BeforeCode
                        })
                    }) {
                        html.push_str(&format!(
                            "<p><a href=\"#{}\">{}</a></p>",
                            source_point_anchor(answers, index, before_index),
                            text(
                                r.locale,
                                "Read the retained before observation",
                                "보존된 이전 관측 읽기"
                            )
                        ));
                    }
                }
                if e.source_representation.as_deref() == Some("bounded_excerpt") {
                    html.push_str(&format!("<p class=\"source-note\">{}</p>",text(r.locale,"Bounded file observation: surrounding code was not retained. Coordinates refer to the repository file; missing context remains unobserved.","제한된 파일 관측: 주변 코드는 보존되지 않았습니다. 좌표는 저장소 파일 기준이며 누락된 맥락은 관측되지 않았습니다.")));
                }
                html.push_str(&format!("<pre class=\"code-region\" tabindex=\"0\" role=\"region\" aria-label=\"{}: {}:{}–{}\"><span class=\"source-line-numbers\" aria-hidden=\"true\">{}</span><code lang=\"\">{}</code></pre>",text(r.locale,"Original Source","Source 원문"),escape(&c.locator),c.start_line,e.displayed_end_line,e.text.split_inclusive('\n').enumerate().map(|(line,_)|format!("<span data-line=\"{}\"></span>",c.start_line+line)).collect::<String>(),escape(&e.text)));
                if e.omitted_utf8_bytes > 0 {
                    html.push_str(&format!(
                        "<p class=\"source-note initial-range-omission\">{}: {} · {}–{} · bytes {}–{}</p>",
                        text(
                            r.locale,
                            "Omitted UTF-8 bytes in this cited range",
                            "이 인용 범위에서 생략된 UTF-8 바이트"
                        ),
                        e.omitted_utf8_bytes,
                        c.start_line,
                        e.displayed_end_line,
                        c.start_byte,
                        e.displayed_end_byte
                    ));
                }
                if let Some(execution) = &e.execution {
                    html.push_str(&format!(
                        "<p class=\"source-note\">{}: {} · {}: {}</p>",
                        text(r.locale, "Observed exit", "관측 종료 코드"),
                        escape(&execution["exit_code"].to_string()),
                        text(r.locale, "Termination", "종료 상태"),
                        escape(&execution["termination"].to_string())
                    ));
                }
            } else {
                empty_state(
                    html,
                    text(
                        r.locale,
                        "This cited range was omitted by the shared code-reading bound.",
                        "이 인용 범위는 공통 코드 읽기 한도로 생략되었습니다.",
                    ),
                );
            }
            html.push_str(&format!("<p class=\"source-note\">{}</p>",text(r.locale,
                "This is the author's cited claim span. Range binding does not verify that each statement follows from this code.",
                "이것은 작성자가 인용한 주장 범위입니다. 범위 연결만으로 각 문장이 이 코드에서 성립하는지 검증되지는 않습니다.")));
            if !snapshot {
                let endpoint = format!(
                    "/citation?subject={}&language={}&realization={}&unit={}&citation={}&offset=0",
                    percent_encode(&provenance.subject.key()),
                    percent_encode(&provenance.language),
                    percent_encode(&provenance.realization_fingerprint),
                    index,
                    n
                );
                html.push_str(&format!("<div class=\"citation-inspection\" data-citation-url=\"{}\" data-realization=\"{}\" data-unit=\"{}\" data-citation=\"{}\"><p class=\"source-note citation-status\" role=\"status\"></p><button type=\"button\" class=\"citation-read\">{}</button></div>",
                    escape(&endpoint),escape(&provenance.realization_fingerprint),index,n,
                    text(r.locale,"Read permitted cited bytes (up to 4,096)","허용된 인용 바이트 읽기 (최대 4,096)")));
            } else {
                html.push_str(&format!("<p class=\"source-note\">{}</p>",text(r.locale,
                    "Offline: only the displayed bytes are included. Additional omitted evidence cannot be retrieved from this document.",
                    "오프라인: 표시된 바이트만 포함됩니다. 이 문서에서는 추가 생략 근거를 가져올 수 없습니다.")));
            }
            html.push_str("<nav class=\"point-others\">");
            for (other, other_c) in answer
                .citations
                .iter()
                .enumerate()
                .filter(|(other, _)| *other != n)
            {
                html.push_str(&format!(
                    "<a href=\"#{}\">{} {} · {}:{}–{}</a>",
                    source_point_anchor(answers, index, other),
                    text(r.locale, "Source", "근거"),
                    other + 1,
                    escape(&other_c.locator),
                    other_c.start_line,
                    other_c.end_line
                ));
            }
            html.push_str(&format!("</nav><details><summary>{}</summary><pre>{}</pre></details></section>",text(r.locale,"Source identity · observed scope","Source identity · 관측 범위"),escape(&format!("output: {}\nsubject: {:?}\ncitation: {c:#?}\nWork revision: {:?}\nobserved at: {:?}",provenance.realization_fingerprint,provenance.subject,excerpt.map(|e|e.work_revision),excerpt.map(|e|e.observed_at_unix_micros)))));
        }
    }
    if answers.omitted_source_excerpt_count > 0 {
        empty_state(
            html,
            &format!(
                "{}: {}",
                text(r.locale, "Omitted cited excerpts", "생략된 인용 발췌"),
                answers.omitted_source_excerpt_count
            ),
        );
    }
    html.push_str("</aside>");
}
fn work_detail(
    html: &mut String,
    r: &ViewerRequest,
    p: &ProjectProjection,
    w: &UnderstandingWork,
    snapshot: bool,
) {
    html.push_str(&format!(
        "<section class=\"snapshot-page\" id=\"work-{}\">",
        w.work_item_id
    ));
    let answers = volicord_projections::work_answers(w, &r.requested_language, r.locale.fixed());
    let mut records = String::new();
    contextual_limits(&mut records, r, p, snapshot);
    let decisions = if snapshot {
        &p.decision_catalog
    } else {
        &p.selected_work_decisions
    };
    let mut related = String::new();
    for d in decisions
        .iter()
        .filter(|d| w.decision_ids.contains(&d.decision.decision_id))
    {
        decision_link(&mut related, r, d, snapshot);
    }
    for candidate in related_learning(p, w.work_item_id) {
        records.push_str(&format!("<article id=\"learning-{}\" data-candidate-id=\"{}\" data-work-id=\"{}\" data-statement-role=\"session-candidate\">",candidate.candidate_id,candidate.candidate_id,w.work_item_id));
        learning_inspection(&mut records, r, candidate);
        records.push_str("</article>");
    }
    for q in p
        .resume
        .open_questions
        .iter()
        .filter(|q| w.open_question_ids.contains(&q.question_id))
    {
        records.push_str(&format!("<p>{}</p>", escape(&q.prompt)));
    }
    if !snapshot {
        records.push_str("<p>");
        link(
            &mut records,
            r,
            ViewerView::Code {
                scope: CodeScope::Work(Some(w.work_item_id)),
                entity: None,
            },
            text(r.locale, "Explore this Work's code", "이 작업의 코드 탐색"),
            None,
        );
        records.push_str("</p>");
    }
    render_document(
        html,
        r,
        &answers,
        Some(w),
        None,
        &records,
        &related,
        snapshot,
    );
    html.push_str("</section>");
}

fn decision_link(
    html: &mut String,
    r: &ViewerRequest,
    d: &volicord_projections::UnderstandingDecision,
    snapshot: bool,
) {
    html.push_str(&format!(
        "<article class=\"decision-summary\" data-decision-id=\"{}\"><h3>",
        d.decision.decision_id
    ));
    link(
        html,
        r,
        ViewerView::Decisions {
            decision: Some(d.decision.decision_id),
        },
        &decision_choice_attribution(&d.decision, r.locale),
        snapshot
            .then(|| format!("decision-{}", d.decision.decision_id))
            .as_deref(),
    );
    html.push_str("</h3><p class=\"badge\">");
    html.push_str(&escape(brief_decision_state_label(
        d.decision.state,
        r.locale,
    )));
    html.push_str("</p><p>");
    html.push_str(&escape(decision_scope_label(
        d.decision.work_scope,
        r.locale,
    )));
    html.push_str("</p></article>");
}
pub(super) fn decision_answers(
    r: &ViewerRequest,
    decision: &BriefDecision,
) -> volicord_projections::QuestionAnswers {
    let mut answers =
        volicord_projections::decision_answers(decision, &r.requested_language, r.locale.fixed());
    if matches!(decision.work_scope, DecisionWorkScope::WorkItem(_)) {
        if let Some(scope) = answers
            .facts
            .iter_mut()
            .find(|a| a.question == "DeclaredScope")
        {
            scope.text = text(
                r.locale,
                "Declared Work scope: the linked Work; inspect evidence for its identity.",
                "선언된 작업 범위: 연결된 작업. 식별자는 근거에서 확인하세요.",
            )
            .into();
        }
    }
    answers
}
fn decision_detail(
    html: &mut String,
    r: &ViewerRequest,
    d: &volicord_projections::UnderstandingDecision,
    p: &ProjectProjection,
    snapshot: bool,
) {
    html.push_str(&format!("<section id=\"decision-{}\" class=\"decision-reading\" data-decision-id=\"{}\" data-decision-scope=\"{}\">", d.decision.decision_id, d.decision.decision_id, decision_scope_key(d.decision.work_scope)));
    let answers = decision_answers(r, &d.decision);
    let mut records = String::new();
    contextual_limits(&mut records, r, p, snapshot);
    heading(
        &mut records,
        3,
        text(
            r.locale,
            "Alternatives and expected trade-offs",
            "대안과 예상 절충",
        ),
    );
    records.push_str("<ul class=\"alternatives\">");
    for alternative in &d.decision.displayed_alternatives {
        list_item(
            &mut records,
            &format!("{}: {}", alternative.label, alternative.consequence),
        );
    }
    records.push_str("</ul>");
    heading(
        &mut records,
        3,
        text(r.locale, "Declared applicability", "선언된 적용 범위"),
    );
    records.push_str("<dl>");
    definition(
        &mut records,
        text(r.locale, "Typed scope", "명시적 범위"),
        decision_scope_label(d.decision.work_scope, r.locale),
    );
    definition(
        &mut records,
        text(r.locale, "Declared paths", "선언된 경로"),
        &d.declared_paths.join(", "),
    );
    definition(
        &mut records,
        text(r.locale, "Original user rationale", "원래 사용자 이유"),
        d.reading
            .user_rationale
            .original_text
            .as_deref()
            .unwrap_or(""),
    );
    definition(
        &mut records,
        text(
            r.locale,
            "Original recommendation rationale",
            "원래 권고 이유",
        ),
        d.reading
            .recommendation_rationale
            .original_text
            .as_deref()
            .unwrap_or(""),
    );
    definition(
        &mut records,
        text(r.locale, "Reading basis", "읽기 근거"),
        &format!("{:?}", d.reading),
    );
    definition(
        &mut records,
        text(
            r.locale,
            "Decision and exact Question basis",
            "결정 및 정확한 질문 근거",
        ),
        &format!("{:?}", d.decision),
    );
    records.push_str("</dl>");
    for gap in d.known_link_gaps.iter().chain(&d.decision.known_limits) {
        empty_state(&mut records, gap);
    }
    let mut related = String::new();
    if let DecisionWorkScope::WorkItem(work) = d.decision.work_scope {
        link(
            &mut related,
            r,
            ViewerView::Work { work: Some(work) },
            text(r.locale, "Read the linked Work →", "연결된 작업 읽기 →"),
            snapshot.then(|| format!("work-{work}")).as_deref(),
        );
    }
    render_document(
        html,
        r,
        &answers,
        None,
        Some(d),
        &records,
        &related,
        snapshot,
    );
    html.push_str("</section>");
}
pub(super) fn analysis_summary(
    html: &mut String,
    r: &ViewerRequest,
    analysis: &volicord_projections::RepositoryAnalysisReading,
    coverage: bool,
) {
    use volicord_projections::RepositoryAnalysisState as State;
    let (key, label, meaning) = match analysis.state {
        State::Absent => ("absent", text(r.locale, "Not analyzed", "분석 없음"), text(r.locale, "No stored repository analysis. Code explanations cannot establish current structure yet.", "저장된 저장소 분석이 없습니다. 코드 설명으로 현재 구조를 확인할 수 없습니다.")),
        State::Current => ("current", text(r.locale, "Current", "최신"), text(r.locale, "Stored analysis matches the compared repository basis within reported coverage.", "저장된 분석은 보고된 범위에서 비교한 저장소 근거와 일치합니다.")),
        State::Partial => ("partial", text(r.locale, "Partial coverage", "일부 범위"), text(r.locale, "Analysis is current but some observed scopes have limited support.", "분석은 최신이지만 관찰한 일부 범위의 지원이 제한됩니다.")),
        State::Stale => ("stale", text(r.locale, "Stale", "오래됨"), text(r.locale, "The repository has changed. Stored code evidence cannot establish current behavior.", "저장소가 변경되었습니다. 저장된 코드 근거로 현재 동작을 확인할 수 없습니다.")),
        State::Failed => ("failed", text(r.locale, "Failed", "실패"), text(r.locale, "An analysis scope or a later explicit analysis attempt failed. Inspect the affected scope and usable remainder.", "분석 범위 또는 이후 명시적 분석 시도가 실패했습니다. 영향 범위와 사용 가능한 나머지를 확인하세요.")),
        State::FreshnessUnknown => ("unknown", text(r.locale, "Freshness unknown", "최신성 알 수 없음"), text(r.locale, "The current repository basis could not be compared. Stored evidence is not confirmed current.", "현재 저장소 근거를 비교할 수 없습니다. 저장된 근거의 최신성이 확인되지 않았습니다.")),
        State::Unavailable => ("unavailable", text(r.locale, "Unavailable", "이용 불가"), text(r.locale, "Stored analysis could not be read. Canonical memory remains available; inspect recovery details.", "저장된 분석을 읽을 수 없습니다. 정식 기억은 계속 이용할 수 있습니다. 복구 상세를 확인하세요.")),
    };
    html.push_str(&format!("<div class=\"analysis-summary\" data-analysis-state=\"{key}\"><p class=\"state\"><strong>{}: {}</strong></p><p>{}</p>", text(r.locale, "Repository analysis", "저장소 분석"), label, meaning));
    html.push_str(&format!(
        "<p data-analysis-freshness=\"{}\">{}: {}</p>",
        analysis
            .freshness
            .as_ref()
            .map(|f| match f.state {
                FreshnessState::Current => "current",
                FreshnessState::Stale => "stale",
                FreshnessState::Unknown => "unknown",
            })
            .unwrap_or("unknown"),
        text(r.locale, "Source freshness", "소스 최신성"),
        analysis
            .freshness
            .as_ref()
            .map(|f| freshness_state_label(f.state, r.locale))
            .unwrap_or(text(r.locale, "unknown", "알 수 없음"))
    ));
    if coverage {
        if analysis.retained_prior_result {
            empty_state(html, text(r.locale, "A prior result is retained; the failed attempt did not replace it or establish current success.", "이전 결과가 유지됩니다. 실패한 시도는 이를 대체하거나 현재 성공을 입증하지 않습니다."));
        }
        if let Some(diagnostic) = &analysis.diagnostic {
            empty_state(html, diagnostic);
        }
        if let Some(error) = &analysis.latest_attempt_error {
            empty_state(
                html,
                &format!(
                    "{}: {error}",
                    text(
                        r.locale,
                        "Latest attempt could not be verified",
                        "최근 시도를 검증할 수 없음"
                    )
                ),
            );
        }
        if let Some(attempt) = &analysis.latest_attempt {
            html.push_str(&format!(
                "<p>{}: {}</p>",
                text(r.locale, "Latest explicit attempt", "최근 명시적 시도"),
                if attempt.failed {
                    text(r.locale, "failed", "실패")
                } else {
                    text(r.locale, "completed", "완료")
                }
            ));
            if let Some(diagnostic) = &attempt.diagnostic {
                empty_state(html, diagnostic);
            }
        } else {
            empty_state(html, text(r.locale, "No verified latest-attempt receipt is available. A stored snapshot alone does not prove an attempt succeeded.", "검증된 최근 시도 기록이 없습니다. 저장된 스냅샷만으로 시도의 성공을 입증할 수 없습니다."));
        }
    }
    html.push_str(&format!("<p class=\"next-action\">{} <code>{}</code>. {}</p>",
        text(r.locale, "Explicit local refresh:", "명시적 로컬 갱신:"), escape(&analysis.refresh_command),
        text(r.locale, "Run in this Project's bound repository with the same Runtime. Then reload this view. Analysis does not regenerate Work explanations.", "같은 런타임으로 이 프로젝트에 연결된 저장소에서 실행한 뒤 화면을 새로 고치세요. 분석은 작업 설명을 재생성하지 않습니다.")));
    if coverage {
        heading(
            html,
            3,
            text(
                r.locale,
                "Observed scopes and coverage",
                "관찰한 범위 및 분석 지원",
            ),
        );
        if analysis.coverage.is_empty() {
            empty_state(
                html,
                text(
                    r.locale,
                    "No observed coverage is available; this is not complete coverage.",
                    "관찰된 분석 범위가 없습니다. 완전한 분석을 뜻하지 않습니다.",
                ),
            );
        }
        html.push_str("<div class=\"coverage-list\">");
        for scope in &analysis.coverage {
            html.push_str(&format!("<article class=\"coverage-scope\" data-state=\"{}\"><h4>{} · {} · {}</h4><p><strong>{}</strong></p>",
                capability_state_key(scope.state), escape(&scope.area),
                scope.language.as_ref().map(|l| escape(&language_label(l,r.locale))).unwrap_or_else(||text(r.locale,"All languages","모든 언어").into()),
                capability_label(scope.capability,r.locale), capability_state_label(scope.state,r.locale)));
            for (label, value) in [
                (text(r.locale, "Why / limit", "이유 / 한계"), &scope.reason),
                (
                    text(r.locale, "What cannot be concluded", "확인할 수 없는 내용"),
                    &scope.consequence,
                ),
                (
                    text(r.locale, "Usable remainder", "사용 가능한 나머지"),
                    &scope.usable_remainder,
                ),
            ] {
                if let Some(value) = value {
                    html.push_str(&format!(
                        "<p><strong>{}:</strong> {}</p>",
                        label,
                        escape(value)
                    ));
                }
            }
            html.push_str(&format!(
                "<details><summary>{}</summary><p>{}: {} · {}: {} · {}: {}</p></details></article>",
                text(r.locale, "Coverage counts", "분석 범위 수"),
                text(r.locale, "Files", "파일"),
                scope.files,
                text(r.locale, "Entities", "엔터티"),
                scope.entities,
                text(r.locale, "Relations", "관계"),
                scope.relations
            ));
        }
        html.push_str("</div><details class=\"analysis-basis\"><summary>");
        html.push_str(text(
            r.locale,
            "Snapshot identity, freshness basis and exact omissions",
            "스냅샷 식별자, 최신성 근거 및 정확한 생략",
        ));
        html.push_str("</summary><pre>");
        html.push_str(&escape(&format!("analysis={:?}\nrepository={:?}\ngenerated_at={:?}\nfreshness={:?}\nomitted_coverage={}\nlatest_attempt={:?}",analysis.analysis_snapshot,analysis.repository_snapshot,analysis.generated_at_unix_micros,analysis.freshness,analysis.omitted_coverage_count,analysis.latest_attempt)));
        html.push_str("</pre></details>");
    }
    html.push_str("</div>");
}
fn code_limit_counts(p: &ProjectProjection) -> (usize, usize) {
    (
        p.answer_issues
            .iter()
            .filter(|i| i.kind != ProjectionIssueKind::Bound)
            .count(),
        p.answer_capability_gaps.len(),
    )
}

fn code_limit_summary(html: &mut String, r: &ViewerRequest, p: &ProjectProjection, snapshot: bool) {
    let (issues, gaps) = code_limit_counts(p);
    use volicord_projections::RepositoryAnalysisState as State;
    let repository = snapshot
        || matches!(
            r.view,
            ViewerView::Code {
                scope: CodeScope::Repository,
                ..
            }
        );
    let blocker = if repository {
        match p.repository_analysis.state {
            State::Absent => Some(text(r.locale, "No stored analysis: current code meaning is unavailable.", "저장된 분석이 없어 현재 코드 의미를 확인할 수 없습니다.")),
            State::Stale => Some(text(r.locale, "The repository has changed. Stored evidence cannot establish current behavior.", "저장소가 변경되었습니다. 저장된 근거로 현재 동작을 확인할 수 없습니다.")),
            State::Failed => Some(text(r.locale, "An analysis scope or latest explicit attempt failed. A retained prior result does not prove that attempt succeeded.", "분석 범위 또는 최근 명시적 시도가 실패했습니다. 유지된 이전 결과는 해당 시도의 성공을 입증하지 않습니다.")),
            State::FreshnessUnknown => Some(text(r.locale, "The current repository basis could not be compared; stored code evidence is not confirmed current.", "현재 저장소 근거를 비교할 수 없어 저장된 코드 근거의 최신성이 확인되지 않습니다.")),
            State::Unavailable => Some(text(r.locale, "Stored analysis is unavailable. Open Analysis for local recovery guidance.", "저장된 분석을 이용할 수 없습니다. 분석에서 로컬 복구 안내를 확인하세요.")),
            State::Current | State::Partial => None,
        }
    } else {
        None
    };
    if issues + gaps == 0 && blocker.is_none() {
        return;
    }
    html.push_str(&format!("<aside class=\"contextual-limits\" data-code-reliability=\"true\" aria-label=\"{}\" data-issue-count=\"{issues}\" data-gap-count=\"{gaps}\"><h3>{}</h3>", text(r.locale, "Code reliability", "코드 신뢰성"), text(r.locale, "Limits of this answer", "이 설명의 한계")));
    let scope = if r.view.detail().entity.is_some() {
        text(r.locale, "Selected entity evidence", "선택한 엔터티 근거")
    } else if matches!(
        r.view,
        ViewerView::Code {
            scope: CodeScope::Work(_),
            ..
        }
    ) {
        text(r.locale, "Selected Work evidence", "선택한 작업 근거")
    } else {
        text(r.locale, "Repository evidence", "저장소 근거")
    };
    html.push_str(&format!(
        "<p>{scope}: {issues} {} · {gaps} {}.</p>",
        text(r.locale, "issues", "문제"),
        text(r.locale, "capability gaps", "분석 지원 부족"),
    ));
    if issues + gaps > 0 {
        html.push_str(&format!(
            "<p><a href=\"#code-limit-details\">{}</a></p>",
            text(r.locale, "Inspect all diagnostics", "모든 진단 확인")
        ));
    }
    if let Some(blocker) = blocker {
        empty_state(html, blocker);
    }
    // Count from the retained selected basis, without merging identities or
    // inferring repository-wide failure from a scoped capability warning.
    let mut states = BTreeMap::new();
    for issue in p
        .answer_issues
        .iter()
        .filter(|i| i.kind != ProjectionIssueKind::Bound)
    {
        *states
            .entry(projection_issue_kind_label(issue.kind, r.locale).to_owned())
            .or_insert(0_usize) += 1;
    }
    for gap in &p.answer_capability_gaps {
        let key = format!(
            "{} / {} / {}",
            capability_label(gap.capability, r.locale),
            capability_state_label(gap.state, r.locale),
            freshness_state_label(gap.freshness.state, r.locale)
        );
        *states.entry(key).or_insert(0_usize) += 1;
    }
    html.push_str(&format!(
        "<p class=\"code-limit-states\">{}</p>",
        escape(
            &states
                .into_iter()
                .map(|(label, count)| format!("{label}: {count}"))
                .collect::<Vec<_>>()
                .join("; ")
        )
    ));
    if p.answer_issues.iter().any(|i| {
        matches!(
            i.kind,
            ProjectionIssueKind::SourceUnavailable
                | ProjectionIssueKind::SourceStale
                | ProjectionIssueKind::WrongProject
        )
    }) {
        empty_state(html, text(r.locale, "Some selected Source evidence is stale or unavailable. It cannot establish current code meaning; supported remainder stays readable.", "선택한 Source 근거 일부가 오래되었거나 이용 불가입니다. 현재 코드 의미를 확인할 수 없으며 근거 있는 나머지는 계속 읽을 수 있습니다."));
    }
    if p.answer_capability_gaps
        .iter()
        .any(|g| g.freshness.state != FreshnessState::Current)
    {
        empty_state(html, text(r.locale, "Current analysis evidence is not confirmed for every affected scope. Stale or unknown evidence cannot establish current behavior.", "영향 범위 모두에서 현재 분석 근거가 확인되지 않습니다. 오래되었거나 최신성을 알 수 없는 근거로 현재 동작을 확인할 수 없습니다."));
    }
    if p.sections.code == volicord_projections::ReadSectionState::Unavailable {
        empty_state(html, text(r.locale, "Code explanation unavailable: stored analysis cannot be read or verified. Canonical memory remains readable; open Analysis for recovery guidance.", "코드 설명 이용 불가: 저장된 분석을 읽거나 검증할 수 없습니다. 정식 기억은 계속 읽을 수 있습니다. 분석에서 복구 안내를 확인하세요."));
    }
    html.push_str("<p>");
    link(
        html,
        r,
        ViewerView::Tools {
            tool: ViewerTool::Status,
        },
        text(r.locale, "Open Analysis", "분석 열기"),
        snapshot.then_some("health"),
    );
    html.push_str("</p></aside>");
}

fn code_limit_details(html: &mut String, r: &ViewerRequest, p: &ProjectProjection, snapshot: bool) {
    let (issues, gaps) = code_limit_counts(p);
    if issues + gaps == 0 {
        return;
    }
    html.push_str(&format!("<details id=\"code-limit-details\" class=\"code-limit-details\" data-issue-count=\"{issues}\" data-gap-count=\"{gaps}\"><summary>{}: {issues} {} · {gaps} {}</summary><ul class=\"gap-list\">", text(r.locale, "Full Code diagnostics", "전체 코드 진단"), text(r.locale, "issues", "문제"), text(r.locale, "capability gaps", "분석 지원 부족")));
    for issue in p
        .answer_issues
        .iter()
        .filter(|i| i.kind != ProjectionIssueKind::Bound)
    {
        html.push_str(&format!("<li data-limit-kind=\"issue\" data-limit-id=\"{}\"><p>{} · {}: {}</p><pre>{}</pre></li>", escape(&issue.identity), projection_issue_kind_label(issue.kind, r.locale), escape(&issue.affected_scope), escape(&issue.reason), escape(&format!("{issue:#?}"))));
    }
    for gap in &p.answer_capability_gaps {
        let area = if gap.area == "." {
            text(
                r.locale,
                "Repository coverage (root)",
                "저장소 분석 범위 (루트)",
            )
        } else {
            &gap.area
        };
        html.push_str(&format!("<li data-limit-kind=\"gap\" data-analysis-id=\"{}\" data-scope=\"{}\"><p>{} · {} · {} · {} · {}: {}</p>", gap.analysis_snapshot, escape(&gap.area), escape(area), escape(&gap.language.as_ref().map(|l| language_label(l, r.locale)).unwrap_or_else(|| text(r.locale, "All languages", "모든 언어").into())), capability_label(gap.capability, r.locale), capability_state_label(gap.state, r.locale), freshness_state_label(gap.freshness.state, r.locale), escape(&gap.reason)));
        html.push_str("<dl>");
        definition(
            html,
            text(r.locale, "What cannot be concluded", "확인할 수 없는 내용"),
            gap.user_visible_consequence.as_deref().unwrap_or(text(
                r.locale,
                "This scope cannot support a complete current code answer.",
                "이 범위로 완전한 현재 코드 설명을 뒷받침할 수 없습니다.",
            )),
        );
        definition(
            html,
            text(r.locale, "Usable remainder", "사용 가능한 나머지"),
            gap.usable_remainder.as_deref().unwrap_or(text(
                r.locale,
                "Canonical memory remains readable.",
                "정식 기억은 계속 읽을 수 있습니다.",
            )),
        );
        html.push_str(&format!(
            "</dl><pre>{}</pre></li>",
            escape(&format!("{gap:#?}"))
        ));
    }
    html.push_str("</ul><p class=\"next-action\">");
    html.push_str(text(
        r.locale,
        "Inspect Analysis for affected coverage and local refresh or recovery guidance.",
        "분석에서 영향받은 범위와 로컬 갱신 또는 복구 안내를 확인하세요.",
    ));
    link(
        html,
        r,
        ViewerView::Tools {
            tool: ViewerTool::Status,
        },
        text(r.locale, "Open Analysis", "분석 열기"),
        snapshot.then_some("health"),
    );
    html.push_str("</p></details>");
}

fn repository_explanation(
    html: &mut String,
    r: &ViewerRequest,
    p: &ProjectProjection,
    snapshot: bool,
) {
    html.push_str("<section class=\"snapshot-page\" id=\"repository-explanation\" aria-label=\"Repository explanation\">");
    let answers = volicord_projections::explanation_answers(
        &p.repository_explanations,
        &r.requested_language,
        r.locale.fixed(),
    );
    render_answers(html, r, &answers, None, snapshot);
    html.push_str("</section>");
}

fn code(
    html: &mut String,
    r: &ViewerRequest,
    p: &ProjectProjection,
    u: &ProjectUnderstanding,
    snapshot: bool,
) {
    section_start(
        html,
        "code",
        text(r.locale, "Code Understanding", "코드 이해"),
    );
    empty_state(
        html,
        match &r.view {
            ViewerView::Code {
                scope: CodeScope::Work(_),
                ..
            } => text(r.locale, "Selected Work context", "선택한 작업 범위"),
            _ => text(r.locale, "Repository context", "저장소 범위"),
        },
    );
    code_limit_summary(html, r, p, snapshot);
    if let Some(work) = &p.selected_work {
        html.push_str("<div class=\"code-work-meaning\">");
        let answers =
            volicord_projections::work_answers(work, &r.requested_language, r.locale.fixed());
        if !snapshot {
            render_answers(html, r, &answers, Some(work), false);
        }
        html.push_str("<p>");
        link(
            html,
            r,
            ViewerView::Work {
                work: Some(work.work_item_id),
            },
            text(
                r.locale,
                "Read the complete Work explanation and grounding",
                "전체 작업 설명과 근거 읽기",
            ),
            snapshot
                .then(|| format!("work-{}", work.work_item_id))
                .as_deref(),
        );
        html.push_str("</p>");
        html.push_str("</div>");
    }
    let entities = u
        .architecture
        .components
        .iter()
        .chain(&p.selected_entity_neighbors)
        .collect::<Vec<_>>();
    if let Some(entity) = &p.selected_entity {
        html.push_str("<div class=\"selected-code\">");
        heading(html, 3, &entity.display_name);
        for explanation in u.deterministic_explanations.iter().filter(|e| {
            e.kind == UnderstandingExplanationKind::Component
                && e.entity_basis.contains(&entity.identity)
        }) {
            render_deterministic_explanation(html, r, explanation);
        }
        render_code_behavior(html, r, entity, &p.selected_entity_relations);
        html.push_str(&format!(
            "<p>{} · {} · {}</p>",
            escape(&entity.locator),
            escape(&code_entity_kind_label(&entity.kind, r.locale)),
            escape(&range_label(entity.source_range.as_ref(), r.locale))
        ));
        html.push_str("<details><summary>");
        html.push_str(text(
            r.locale,
            "Source locator and retained range",
            "Source 위치 및 보존된 범위",
        ));
        html.push_str("</summary><dl>");
        definition(html, "Source", &entity.source_id.to_string());
        definition(
            html,
            "Analysis Snapshot",
            &entity.analysis_snapshot.to_string(),
        );
        definition(
            html,
            "Repository Snapshot",
            &entity.repository_snapshot.to_string(),
        );
        definition(
            html,
            text(r.locale, "Freshness", "최신성"),
            &format!("{:?}", entity.freshness),
        );
        definition(
            html,
            text(r.locale, "Range", "범위"),
            &format!("{:?}", entity.source_range),
        );
        if let Some(source) = p
            .source_catalog
            .iter()
            .find(|s| s.source.id == entity.source_id)
        {
            definition(
                html,
                text(r.locale, "Retained Source", "보존된 Source"),
                &format!("{source:?}"),
            );
        }
        html.push_str("</dl></details>");
        for (label, incoming) in [
            (
                text(r.locale, "Incoming relationships", "들어오는 관계"),
                true,
            ),
            (
                text(r.locale, "Outgoing relationships", "나가는 관계"),
                false,
            ),
        ] {
            heading(html, 4, label);
            for relation in p.selected_entity_relations.iter().filter(|rel| {
                if incoming {
                    rel.target_entity.as_deref() == Some(entity.identity.as_str())
                } else {
                    rel.source_entity == entity.identity
                }
            }) {
                relation_detail(html, r, relation, &entities, snapshot);
            }
        }
        if p.omitted_selected_relation_count > 0 {
            empty_state(html, text(r.locale, "Some relationships are outside this displayed neighborhood. Inspect the exact bound below.", "일부 관계가 표시된 이웃 범위 밖에 있습니다. 아래에서 정확한 제한을 확인하세요."));
            html.push_str(&format!("<details class=\"relationship-bounds\"><summary>{}</summary><p>{} {}</p></details>", text(r.locale, "Relationship display bounds", "관계 표시 범위"), p.omitted_selected_relation_count, text(r.locale, "relationships omitted", "관계 생략")));
        }
        html.push_str("</div>");
    }
    if r.view.detail().entity.is_some()
        && p.selected_entity.is_none()
        && p.sections.code == volicord_projections::ReadSectionState::Unavailable
    {
        empty_state(html, text(r.locale, "Entity detail unavailable: stored analysis cannot verify the requested entity identity. Canonical Work remains readable.", "엔터티 상세 이용 불가: 저장된 분석으로 요청한 엔터티 identity를 검증할 수 없습니다. Canonical 작업은 계속 읽을 수 있습니다."));
    }
    empty_state(
        html,
        text(
            r.locale,
            "Static import, reference and syntactic call evidence does not confirm runtime flow.",
            "정적 import, 참조 및 구문 호출 근거는 런타임 흐름을 확정하지 않습니다.",
        ),
    );
    if p.selected_entity.is_some() {
        empty_state(html, text(r.locale, "Diagram focus: selected entity and its bounded stored incoming/outgoing relationships.", "다이어그램 범위: 선택한 엔터티와 제한된 저장 incoming/outgoing 관계."));
    }
    heading(
        html,
        3,
        text(
            r.locale,
            "How the architecture and code connect",
            "아키텍처와 코드의 연결",
        ),
    );
    html.push_str(&format!("<div class=\"fact-legend\"><span data-statement-role=\"verified-fact\">{}</span><span data-statement-role=\"deterministic-derived\">{}</span><span data-statement-role=\"generated-interpretation\">{}</span></div>",text(r.locale,"Structural / semantic facts","구조 / 의미 사실"),text(r.locale,"Deterministic explanations","결정론적 설명"),text(r.locale,"Generated interpretations (separate)","생성 해석 (별도)")));
    for explanation in &u.deterministic_explanations {
        if !p.selected_entity.as_ref().is_some_and(|entity| {
            explanation.kind == UnderstandingExplanationKind::Component
                && explanation.entity_basis.contains(&entity.identity)
        }) {
            render_deterministic_explanation(html, r, explanation);
        }
    }
    for entity in &u.architecture.components {
        if p.selected_entity
            .as_ref()
            .is_none_or(|selected| selected.identity != entity.identity)
        {
            render_code_behavior(html, r, entity, &u.architecture.relationships);
        }
    }
    for interpretation in &u.generated_interpretations {
        let prose = if matches!(
            interpretation.state,
            volicord_projections::CodeExplanationState::Unavailable
                | volicord_projections::CodeExplanationState::Unsupported
        ) {
            text(
                r.locale,
                "Generated prose is unavailable for this evidence basis.",
                "이 근거로 생성 해석 본문을 제공할 수 없습니다.",
            )
        } else {
            &interpretation.text
        };
        html.push_str(&format!("<details data-statement-role=\"generated-interpretation\" data-explanation-state=\"{:?}\"><summary>{} · {:?}</summary><p>{}</p><p>{}</p><pre>{}</pre></details>", interpretation.state, escape(text(r.locale,"Generated interpretation","생성 해석")), interpretation.state, escape(prose), escape(&interpretation.known_gaps.join("; ")), escape(&format!("producer: {}\ngenerated: {}\nSource: {:?}\nAnalysis: {}\nRepository: {}\nentities: {:?}\nrelations: {:?}\nranges: {:?}\nuncertainty: {:?}", interpretation.producer, interpretation.generated_at_unix_micros, interpretation.source_basis, interpretation.analysis_snapshot, interpretation.repository_snapshot, interpretation.entity_basis, interpretation.relation_basis, interpretation.source_ranges, interpretation.uncertainty))));
    }
    code_limit_details(html, r, p, snapshot);
    // Repository-wide status belongs to Analysis; only Work-relevant gaps accompany code.
    if matches!(
        r.view,
        ViewerView::Code {
            scope: CodeScope::Repository,
            ..
        }
    ) || snapshot
    {
        analysis_summary(html, r, &p.repository_analysis, false);
    }
    if !u.architecture.components.is_empty() || !u.architecture.relationships.is_empty() {
        render_grounded_diagram(
            html,
            r,
            u,
            "architecture-topology",
            text(
                r.locale,
                "Components and dependencies",
                "컴포넌트 및 의존성",
            ),
            |_| true,
            true,
        );
    } else {
        empty_state(html, text(r.locale, "No supported connection is available in this scope. Components below establish structure only; use explicit local analysis to inspect relationships.", "이 범위에 근거 있는 연결이 없습니다. 아래 컴포넌트는 구조만 보여 줍니다. 명시적 로컬 분석으로 관계를 확인하세요."));
    }
    html.push_str(&format!("<p class=\"relationship-guide\">{}</p>", text(r.locale, "Arrows follow the recorded source → target direction. Containment and dependency describe structure; references describe symbol use. Only syntactic calls support the static call figure, which does not prove runtime or data flow.", "화살표는 기록된 출발 → 대상 방향을 따릅니다. 포함과 의존은 구조, 참조는 심볼 사용을 설명합니다. 구문 호출만 정적 호출 그림의 근거이며 런타임 또는 데이터 흐름을 입증하지 않습니다.")));
    html.push_str(&format!(
        "<div class=\"flow-support\" data-flow-state=\"{:?}\">",
        u.architecture.flow_evidence.state
    ));
    if !u.architecture.flow_evidence.missing_evidence.is_empty() {
        html.push_str("<details><summary>");
        html.push_str(text(
            r.locale,
            "Exact missing relationship evidence",
            "정확한 관계 근거 부족",
        ));
        html.push_str("</summary><ul>");
        for limit in &u.architecture.flow_evidence.missing_evidence {
            list_item(html, limit);
        }
        html.push_str("</ul></details>");
    }
    html.push_str("</div>");
    render_grounded_diagram(
        html,
        r,
        u,
        "flow-topology",
        text(r.locale, "Static call relationships", "정적 호출 관계"),
        is_flow_relation,
        false,
    );
    if u.architecture.flow_evidence.relation_ids.is_empty() {
        html.push_str(&format!("<p class=\"next-action\">{} <code>{}</code>. {}</p>", text(r.locale,"Inspect available call support with","호출 분석 지원 확인:"),escape(&p.repository_analysis.refresh_command),text(r.locale,"Missing resolved calls cannot establish execution or data flow; analysis may still lack this capability.","해결된 호출이 없으면 실행 또는 데이터 흐름을 확인할 수 없습니다. 분석 후에도 이 기능의 지원이 없을 수 있습니다.")));
    }
    heading(
        html,
        3,
        text(
            r.locale,
            "Entities and relationships (list path)",
            "엔터티 및 관계 (목록 탐색)",
        ),
    );
    for entity in &u.architecture.components {
        html.push_str(&format!("<details id=\"entity-{}\" data-entity-id=\"{}\"><summary>{} — {}</summary><p>{}</p><p>{}</p>",fragment_identity(&entity.identity),escape(&entity.identity),escape(&entity.display_name),escape(&entity.locator),escape(&code_entity_kind_label(&entity.kind,r.locale)),escape(&range_label(entity.source_range.as_ref(),r.locale))));
        if !snapshot {
            let mut view = r.view.clone();
            if let ViewerView::Code {
                entity: selected, ..
            } = &mut view
            {
                *selected = Some(entity.identity.clone());
            }
            link(
                html,
                r,
                view,
                text(
                    r.locale,
                    "Inspect entity and incoming / outgoing relationships",
                    "엔터티 및 들어오는 / 나가는 관계 확인",
                ),
                None,
            );
        }
        html.push_str("</details>");
    }
    for relation in u
        .architecture
        .relationships
        .iter()
        .chain(&u.evidence.unresolved_relationships)
    {
        relation_detail(html, r, relation, &entities, snapshot);
    }
    render_understanding_evidence(html, r, u);
    section_end(html);
}

fn render_code_behavior(
    html: &mut String,
    r: &ViewerRequest,
    entity: &MapEntity,
    relations: &[MapRelation],
) {
    let behavior = &entity.behavior;
    let explanation = behavior.explanation(&entity.display_name, r.locale == ViewerLocale::Korean);
    html.push_str(&format!("<div data-statement-role=\"deterministic-derived\" data-code-behavior=\"{}\" data-explanation-state=\"{:?}\"><p>{}: {} · {}</p>", escape(&entity.identity), behavior.state, text(r.locale, "Source-bound behavior", "소스에 연결된 동작"), escape(&entity.display_name), code_explanation_state_label(behavior.state, r.locale)));
    if !matches!(
        behavior.state,
        volicord_projections::CodeExplanationState::Current
            | volicord_projections::CodeExplanationState::Partial
    ) {
        html.push_str(&format!(
            "<p>{}</p>",
            text(
                r.locale,
                "Current behavior cannot be established from this evidence.",
                "이 근거로 현재 동작을 확인할 수 없습니다."
            )
        ));
    }
    if !explanation.is_empty() {
        html.push_str(&format!("<p>{}</p>", escape(&explanation)));
    }
    html.push_str(&format!("<p>{}</p><details><summary>{}</summary><pre>{}</pre></details></div>", escape(&behavior.limitations.join("; ")), text(r.locale, "Body expressions and exact source evidence", "본문 표현식과 정확한 소스 근거"), escape(&format!("Source: {}\nAnalysis: {}\nRepository: {}\nfreshness: {:?}\nclaims: {:#?}\nomitted: {}\nsupporting static relations: {:?}", entity.source_id, entity.analysis_snapshot, entity.repository_snapshot, entity.freshness, behavior.claims, behavior.omitted_count, relations.iter().filter(|relation| relation.source_entity == entity.identity || relation.target_entity.as_deref() == Some(entity.identity.as_str())).map(|relation| (&relation.identity, &relation.kind, relation.class, &relation.freshness)).collect::<Vec<_>>()))));
}
pub(super) fn fragment_identity(id: &str) -> String {
    id.as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
pub(super) fn surface(
    html: &mut String,
    r: &ViewerRequest,
    p: &ProjectProjection,
    u: &ProjectUnderstanding,
) {
    match &r.view {
        ViewerView::Overview => overview(html, r, p, u, false),
        ViewerView::Work { work: Some(_) } => {
            if let Some(w) = &p.selected_work {
                work_detail(html, r, p, w, false);
            }
        }
        ViewerView::Work { work: None } | ViewerView::WorkPage { .. } => {
            work_index(html, r, p, false);
        }
        ViewerView::Decisions { decision: Some(_) } => {
            if let Some(d) = &p.selected_decision {
                decision_detail(html, r, d, p, false);
            }
        }
        ViewerView::Decisions { decision: None } | ViewerView::DecisionsPage { .. } => {
            section_start(
                html,
                "decision-reading",
                text(r.locale, "Decisions", "결정"),
            );
            if p.decision_count == 0 {
                empty_state(
                    html,
                    text(
                        r.locale,
                        "No Decisions have been recorded.",
                        "기록된 결정이 없습니다.",
                    ),
                );
            }
            for d in &p.decision_catalog {
                decision_link(html, r, d, false);
            }
            pagination(
                html,
                r,
                p.decision_count,
                r.view.detail().decision_page,
                false,
            );
            section_end(html);
        }
        ViewerView::RepositoryExplanation => repository_explanation(html, r, p, false),
        ViewerView::Code { .. } => code(html, r, p, u, false),
        ViewerView::Tools { .. } => {}
    }
}
pub(super) fn snapshot(
    html: &mut String,
    r: &ViewerRequest,
    p: &ProjectProjection,
    u: &ProjectUnderstanding,
) {
    overview(html, r, p, u, true);
    work_index(html, r, p, true);
    let mut seen = std::collections::BTreeSet::new();
    for w in p
        .work_history
        .iter()
        .chain(p.selected_work.iter())
        .chain(&u.work_overview.current.items)
        .chain(&u.work_overview.completed.items)
        .chain(&u.work_overview.remaining.items)
        .chain(&u.work_overview.next_steps.items)
    {
        if seen.insert(w.work_item_id) {
            work_detail(html, r, p, w, true);
        }
    }
    html.push_str("<section class=\"snapshot-page\" id=\"decision-reading\">");
    for d in &p.decision_catalog {
        decision_detail(html, r, d, p, true);
    }
    html.push_str("</section>");
    repository_explanation(html, r, &p.for_repository_reading(), true);
    let repository_projection = p.for_repository_reading();
    let repository = build_project_understanding(
        &repository_projection,
        UnderstandingBound {
            max_items_per_section: 32,
        },
    );
    html.push_str("<section class=\"snapshot-page\" id=\"repository-code\">");
    code(html, r, &repository_projection, &repository, true);
    html.push_str("</section>");
}
fn work_index(html: &mut String, r: &ViewerRequest, p: &ProjectProjection, snapshot: bool) {
    html.push_str(
        "<section class=\"snapshot-page\" id=\"works\"><p class=\"eyebrow\">VOLICORD / WORK</p>",
    );
    html.push_str(&format!("<h1 class=\"index-title\">{}</h1><p class=\"intro index-intro\">{}</p><div class=\"work-list\">",text(r.locale,"Work","작업"),text(r.locale,"Read the work and its recorded progress.","작업의 내용과 진행 현황을 확인합니다.")));
    for w in &p.work_history {
        work_summary(html, r, w, snapshot);
    }
    if p.work_count == 0 {
        empty_state(
            html,
            text(
                r.locale,
                "No Work has been recorded.",
                "기록된 작업이 없습니다.",
            ),
        );
    }
    html.push_str("</div>");
    if !snapshot {
        pagination(html, r, p.work_count, r.view.detail().work_page, true);
    }
    html.push_str("</section>");
}

fn pagination(html: &mut String, r: &ViewerRequest, count: usize, page: usize, work: bool) {
    html.push_str("<nav aria-label=\"List pages\">");
    for next in [
        page.checked_sub(1),
        page.checked_add(1)
            .filter(|next| next.saturating_mul(64) < count),
    ]
    .into_iter()
    .flatten()
    {
        let view = if work {
            ViewerView::WorkPage { page: next }
        } else {
            ViewerView::DecisionsPage { page: next }
        };
        link(
            html,
            r,
            view,
            &format!("{} {}", text(r.locale, "Page", "페이지"), next + 1),
            None,
        );
    }
    html.push_str("</nav>");
}

fn relation_detail(
    html: &mut String,
    r: &ViewerRequest,
    relation: &MapRelation,
    entities: &[&MapEntity],
    snapshot: bool,
) {
    let endpoint = |id: &str| entities.iter().copied().find(|e| e.identity == id);
    use volicord_projections::CodeRelationshipRole as Role;
    let (role_key, role_label) = match relation.role() {
        Role::Containment => (
            "containment",
            text(r.locale, "Structure / containment", "구조 / 포함"),
        ),
        Role::Dependency => (
            "dependency",
            text(r.locale, "Structure / dependency", "구조 / 의존"),
        ),
        Role::SyntacticCall => (
            "syntactic-call",
            text(r.locale, "Static syntactic call", "정적 구문 호출"),
        ),
        Role::SymbolReference => ("reference", text(r.locale, "Symbol reference", "심볼 참조")),
        Role::TypeRelationship => ("type", text(r.locale, "Type relationship", "타입 관계")),
        Role::Other => (
            "other",
            text(r.locale, "Other stored relationship", "기타 저장된 관계"),
        ),
    };
    html.push_str(&format!("<div class=\"relationship-item\" data-relationship-role=\"{role_key}\"><p><strong>{role_label}</strong></p>"));

    html.push_str(&format!("<details class=\"relationship\" data-relation-id=\"{}\" data-relation-class=\"{}\"><summary>{}: {} → {}</summary>",escape(&relation.identity),map_relation_class_key(relation.class),escape(&relation.kind),escape(&endpoint(&relation.source_entity).map(|e|format!("{} — {}",e.display_name,e.locator)).unwrap_or_else(||text(r.locale,"Source endpoint unavailable","출발 엔터티 없음").into())),escape(&relation.target_entity.as_deref().and_then(endpoint).map(|e|format!("{} — {}",e.display_name,e.locator)).unwrap_or_else(||format!("{}: {}",text(r.locale,"Unresolved / unavailable target","미해결 / 이용 불가 대상"),relation.unresolved_target.as_deref().unwrap_or("entity omitted"))))));
    for id in [&relation.source_entity]
        .into_iter()
        .chain(relation.target_entity.as_ref())
    {
        if let Some(entity) = endpoint(id) {
            if !snapshot {
                html.push_str("<p>");
                link(
                    html,
                    r,
                    ViewerView::Code {
                        scope: CodeScope::Repository,
                        entity: Some(entity.identity.clone()),
                    },
                    &format!(
                        "{}: {}",
                        text(r.locale, "Open repository entity", "저장소 엔터티 열기"),
                        entity.display_name
                    ),
                    None,
                );
                html.push_str("</p>");
            } else {
                html.push_str(&format!(
                    "<p>{} — {}</p>",
                    escape(&entity.display_name),
                    escape(&entity.locator)
                ));
            }
        }
    }
    html.push_str("<dl>");
    definition(
        html,
        text(r.locale, "Evidence class", "근거 분류"),
        match relation.class {
            MapRelationClass::StructuralFact => text(r.locale, "Structural fact", "구조 사실"),
            MapRelationClass::SemanticResult => text(r.locale, "Semantic result", "의미 분석 결과"),
        },
    );
    definition(
        html,
        text(r.locale, "Freshness", "최신성"),
        &format!("{:?}", relation.freshness.state),
    );
    definition(
        html,
        text(r.locale, "Supporting range", "근거 범위"),
        &range_label(relation.supporting_range.as_ref(), r.locale),
    );
    html.push_str("</dl><details><summary>");
    html.push_str(text(
        r.locale,
        "Exact relation and Source evidence",
        "정확한 관계 및 Source 근거",
    ));
    html.push_str("</summary><p>");
    html.push_str(&escape(&format!("{relation:?}")));
    html.push_str("</p></details></details></div>");
}

fn range_label(
    range: Option<&volicord_repository_intelligence::SourceRange>,
    locale: ViewerLocale,
) -> String {
    range
        .map(|r| {
            format!(
                "{} :{}:{}–{}:{} ({:?}; {:?}){}",
                r.locator,
                r.start.line,
                r.start.column,
                r.end.line,
                r.end.column,
                r.coordinate_convention,
                r.meaning,
                r.precision_limit
                    .as_ref()
                    .map(|limit| format!("; {limit}"))
                    .unwrap_or_default()
            )
        })
        .unwrap_or_else(|| text(locale, "Source range unavailable", "Source 범위 없음").into())
}

fn related_learning(
    p: &ProjectProjection,
    work: ContextItemId,
) -> Vec<&volicord_projections::CandidateInspection> {
    p.candidate_inspection
        .iter()
        .filter(|candidate| {
            candidate
                .learning_deliberation
                .as_ref()
                .is_some_and(|learning| learning.goal_context_id == work)
        })
        .collect()
}

pub(super) fn learning_inspection(
    html: &mut String,
    r: &ViewerRequest,
    candidate: &volicord_projections::CandidateInspection,
) {
    let Some(basis) = &candidate.learning_explanation_basis else {
        return;
    };
    html.push_str(
        "<div class=\"learning-candidate-inspection\" data-statement-role=\"session-candidate\">",
    );
    empty_state(html, text(r.locale,
        "Learning participation and implementation selection remain Session Candidate context; they do not grant canonical Decision authority.",
        "학습 참여와 구현 선택은 Session Candidate 맥락이며 canonical Decision 권한을 부여하지 않습니다."));
    html.push_str("<dl>");
    definition(
        html,
        text(r.locale, "Content representation", "내용 표현"),
        text(
            r.locale,
            "Retained quotations in their original language",
            "원래 언어로 보존된 인용",
        ),
    );
    definition(
        html,
        text(r.locale, "Availability", "이용 가능성"),
        match basis.availability {
            volicord_projections::LearningExplanationAvailability::Available => {
                text(r.locale, "Available", "이용 가능")
            }
            volicord_projections::LearningExplanationAvailability::Degraded => text(
                r.locale,
                "Degraded; inspect the gaps below",
                "부분 이용 가능; 아래 빈틈 확인",
            ),
            volicord_projections::LearningExplanationAvailability::Unavailable => {
                text(r.locale, "Unavailable", "이용 불가")
            }
        },
    );
    html.push_str("</dl>");
    if let Some(problem) = &basis.problem {
        heading(
            html,
            4,
            text(r.locale, "Recorded learning problem", "기록된 학습 문제"),
        );
        html.push_str(&format!("<p>{}</p>", escape(problem)));
    }
    html.push_str("<ul>");
    for fact in &basis.established_facts {
        list_item(html, fact);
    }
    html.push_str("</ul>");
    for alternative in &basis.alternatives {
        html.push_str("<details><summary>");
        html.push_str(&escape(&format!(
            "{}: {}",
            alternative.choice_summary, alternative.alternative_summary
        )));
        html.push_str("</summary><ul>");
        for consequence in &alternative.technical_consequences {
            list_item(html, consequence);
        }
        html.push_str("</ul></details>");
    }
    html.push_str("<dl>");
    use volicord_projections::LearningSelectionOutcome;
    let outcome = match &basis.selection_outcome {
        LearningSelectionOutcome::Selected {
            selections,
            completed,
        } => {
            let labels = selections
                .iter()
                .map(|s| {
                    basis
                        .alternatives
                        .iter()
                        .find(|a| {
                            a.choice_id == s.choice_id && a.alternative_id == s.alternative_id
                        })
                        .map_or_else(
                            || {
                                text(
                                    r.locale,
                                    "Selected alternative content unavailable",
                                    "선택한 대안 내용 이용 불가",
                                )
                                .to_owned()
                            },
                            |a| a.alternative_summary.clone(),
                        )
                })
                .collect::<Vec<_>>()
                .join("; ");
            format!(
                "{}: {labels}",
                if *completed {
                    text(r.locale, "Completed learning selection", "완료된 학습 선택")
                } else {
                    text(
                        r.locale,
                        "Selected; awaiting completion",
                        "선택됨; 완료 대기",
                    )
                }
            )
        }
        LearningSelectionOutcome::NotRecorded => {
            text(r.locale, "Response not recorded", "응답 기록 없음").into()
        }
        LearningSelectionOutcome::Delegated => text(
            r.locale,
            "Delegated implementation selection",
            "구현 선택 위임",
        )
        .into(),
        LearningSelectionOutcome::Skipped => {
            text(r.locale, "Skipped learning interaction", "학습 대화 건너뜀").into()
        }
        LearningSelectionOutcome::ResearchOrPrototypeRequired { .. } => text(
            r.locale,
            "Research or prototype evidence required",
            "조사 또는 프로토타입 근거 필요",
        )
        .into(),
        LearningSelectionOutcome::ReconsiderationRequested => {
            text(r.locale, "Reconsideration requested", "재검토 요청됨").into()
        }
    };
    definition(
        html,
        text(r.locale, "Response / selection", "응답 / 선택"),
        &outcome,
    );
    definition(
        html,
        text(r.locale, "User rationale", "사용자 이유"),
        basis.latest_user_rationale.as_deref().unwrap_or(text(
            r.locale,
            "Not recorded",
            "기록되지 않음",
        )),
    );
    definition(
        html,
        text(r.locale, "Agent feedback", "에이전트 피드백"),
        basis.latest_agent_feedback.as_deref().unwrap_or(text(
            r.locale,
            "Not recorded",
            "기록되지 않음",
        )),
    );
    if let Some(recommendation) = &basis.latest_agent_recommendation {
        definition(
            html,
            text(r.locale, "Agent recommendation", "에이전트 권고"),
            &recommendation.rationale,
        );
    }
    html.push_str("</dl><ul>");
    for reason in basis
        .availability_reasons
        .iter()
        .chain(&basis.remaining_uncertainty)
    {
        list_item(html, reason);
    }
    html.push_str("</ul><details><summary>");
    html.push_str(text(
        r.locale,
        "Inspect retained rounds and grounding",
        "보존된 라운드와 근거 확인",
    ));
    html.push_str("</summary><pre>");
    html.push_str(&escape(&format!(
        "Candidate {} revision {:?}\n{basis:?}\n{:?}",
        candidate.candidate_id, candidate.revision, candidate.learning_deliberation
    )));
    html.push_str("</pre></details></div>");
}

#[cfg(test)]
#[path = "reading_tests.rs"]
mod reading_tests;
