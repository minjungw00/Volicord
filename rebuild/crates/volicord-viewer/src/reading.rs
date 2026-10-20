//! Purpose-oriented sections shared by live and whole-snapshot rendering.
use super::*;
use crate::ViewerView;

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
            text(r.locale, "Repository explanation", "저장소 설명"),
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
    html.push_str("</nav>");
}

pub(super) fn contextual_limits(
    html: &mut String,
    r: &ViewerRequest,
    p: &ProjectProjection,
    _snapshot: bool,
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
        "Use volicord status and volicord doctor for detailed coverage and recovery.",
        "자세한 범위와 복구는 volicord status와 volicord doctor로 확인하세요.",
    ));
    html.push_str("</p></aside>");
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
    if snapshot {
        html.push_str(&format!("<p class=\"snapshot-link-help source-note\">{}</p>",text(r.locale,
        "Choose a page above. If a saved section link brought you here, that section is unavailable in this snapshot.",
        "위에서 읽을 문서를 선택하세요. 저장한 문단 링크로 이곳에 왔다면 이 스냅샷에서 해당 문단을 사용할 수 없습니다.")));
    }
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
    let publication_attributes = answers
        .work_name
        .iter()
        .map(|scope| ("name", scope))
        .chain(answers.work_summary.iter().map(|scope| ("summary", scope)))
        .map(|(name, scope)| {
            format!(
                " data-{name}-state=\"{:?}\" data-{name}-realization=\"{}\"",
                scope.explanation_state,
                scope
                    .provenance
                    .as_ref()
                    .map(|p| p.realization_fingerprint.as_str())
                    .unwrap_or_default()
            )
        })
        .collect::<String>();
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
    html.push_str(&format!("<article class=\"work-card\" data-work-id=\"{}\" data-work-state=\"{}\"{}><div class=\"work-card-heading\"><h3 lang=\"{}\" data-title-basis=\"{}\">{}</h3><span class=\"work-status\">{}</span></div>",w.work_item_id,understanding_work_state_key(w.state),publication_attributes,escape(&language),basis,escape(&title),understanding_work_state_label(w.state,r.locale)));
    if let Some(synopsis) = answers.card_summary() {
        html.push_str(&format!("<p class=\"work-summary\" data-question=\"Synopsis\" lang=\"{}\" data-content-language=\"{}\">{}</p>",escape(&content_language_tag(&r.requested_language)),escape(&r.requested_language),escape(&synopsis.text)));
        if answers.card_name().is_none() {
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
            "Work summary is unavailable. Read the original instructions and available records in Work detail.",
            "작업 요약을 사용할 수 없습니다. 작업 상세에서 원래 지시와 사용 가능한 기록을 읽을 수 있습니다.")));
    }
    if answers.explanation_state != volicord_projections::ExplanationState::Current {
        html.push_str(&format!("<p class=\"content-unavailable\" data-publication-scope=\"article\" data-publication-state=\"{:?}\">{}</p>",answers.explanation_state,text(r.locale,"Technical detail is unavailable; valid card content remains readable.","상세 설명을 사용할 수 없습니다. 유효한 카드 내용은 계속 읽을 수 있습니다.")));
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
    if let Some(title) = answers.card_name() {
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
/// Rendering identity is separate from Source/citation authority. In particular,
/// unavailable publications have no provenance but still have a subject and scope.
struct ReaderAnswers<'a> {
    answers: &'a volicord_projections::QuestionAnswers,
    namespace: String,
    subject: String,
    project: ProjectId,
    language: String,
}
impl std::ops::Deref for ReaderAnswers<'_> {
    type Target = volicord_projections::QuestionAnswers;
    fn deref(&self) -> &Self::Target {
        self.answers
    }
}
impl<'a> ReaderAnswers<'a> {
    fn new(
        answers: &'a volicord_projections::QuestionAnswers,
        project: ProjectId,
        subject: &str,
        language: &str,
        scope: &str,
    ) -> Self {
        // A tuple avoids delimiter ambiguity; hashing bounds IDs and excludes
        // hostile strings from HTML control names. Never derive identity from prose.
        let basis = serde_json::json!([
            project.to_string(),
            subject,
            scope,
            language,
            format!("{:?}", answers.explanation_state),
            answers
                .provenance
                .as_ref()
                .map(|p| (&p.subject, &p.realization_fingerprint))
        ]);
        Self {
            answers,
            namespace: volicord_projections::explanation_text_hash(&basis.to_string())
                .trim_start_matches("sha256:")
                .into(),
            subject: subject.into(),
            project,
            language: language.into(),
        }
    }
    fn publication<'b>(
        &self,
        answers: &'b volicord_projections::QuestionAnswers,
        scope: &str,
    ) -> ReaderAnswers<'b> {
        ReaderAnswers::new(answers, self.project, &self.subject, &self.language, scope)
    }
}
fn reader_claim_anchor(answers: &ReaderAnswers<'_>, index: usize) -> String {
    format!("reader-{}-p{}", answers.namespace, index + 1)
}
fn source_point_anchor(answers: &ReaderAnswers<'_>, unit: usize, citation: usize) -> String {
    format!("{}-c{}", reader_claim_anchor(answers, unit), citation + 1)
}
fn reader_citations(
    html: &mut String,
    r: &ViewerRequest,
    answers: &ReaderAnswers<'_>,
    index: usize,
) {
    if answers.prose[index].citations.is_empty() {
        return;
    }
    html.push_str("<div class=\"evidence-actions\">");
    for (n, c) in answers.prose[index].citations.iter().enumerate() {
        let label = format!(
            "{} · {} {} ↗ · {}:{}–{}",
            citation_use_label(r.locale, c.use_kind),
            text(r.locale, "Source", "근거"),
            n + 1,
            c.locator,
            c.start_line,
            c.end_line
        );
        html.push_str(&format!("<a class=\"evidence-link\" data-origin=\"{}\" href=\"#{}\" aria-label=\"{}\" title=\"{}\">{} {} ↗</a>",reader_claim_anchor(answers,index),source_point_anchor(answers,index,n),escape(&label),escape(&label),text(r.locale,"Source","근거"),n+1));
    }
    html.push_str("</div>");
}

fn document_anchor(answers: &ReaderAnswers<'_>, section: &str) -> String {
    format!("{}-{section}", reader_claim_anchor(answers, 0))
}
fn reader_unit(html: &mut String, r: &ViewerRequest, answers: &ReaderAnswers<'_>, index: usize) {
    let answer = &answers.prose[index];
    let outer = match answer.question.as_str() {
        "ComparisonCondition" => "dt",
        "ComparisonResult" => "dd",
        "FlowStep" => "li",
        _ => "div",
    };
    let tag = if answer.question == "SectionHeading" {
        if answers.sections.iter().any(|s| {
            s.heading_index == Some(index) && s.layer != volicord_projections::ArticleLayer::Article
        }) {
            "h3"
        } else {
            "h2"
        }
    } else {
        "p"
    };
    html.push_str(&format!("<{outer} class=\"unit\" id=\"{}\" tabindex=\"-1\"><{tag} class=\"unit-text\" data-question=\"{}\" lang=\"{}\" data-content-language=\"{}\">{}</{tag}>",reader_claim_anchor(answers,index),escape(&answer.question),escape(&content_language_tag(&r.requested_language)),escape(&r.requested_language),escape(&answer.text)));
    reader_citations(html, r, answers, index);
    html.push_str(&format!("</{outer}>"));
}

/// Consume the shared producer/read structure without renumbering any claim.
fn render_article_layer(
    html: &mut String,
    r: &ViewerRequest,
    answers: &ReaderAnswers<'_>,
    layer: volicord_projections::ArticleLayer,
) {
    for section in answers.sections.iter().filter(|s| s.layer == layer) {
        html.push_str("<section class=\"article-section\">");
        if let Some(index) = section.heading_index {
            reader_unit(html, r, answers, index);
        }
        let mut flow_open = false;
        for &index in &section.unit_indices {
            let answer = &answers.prose[index];
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
        html.push_str("</section>");
    }
}
fn render_card_publication(
    html: &mut String,
    r: &ViewerRequest,
    answers: &ReaderAnswers<'_>,
    scope: &str,
) {
    if answers.provenance.is_none() {
        html.push_str(&format!("<p class=\"publication-unavailable source-note\" data-publication-scope=\"{scope}\" data-publication-state=\"{:?}\">{}</p>",answers.explanation_state,
            text(r.locale,"Work summary is unavailable.","작업 요약을 사용할 수 없습니다.")));
        return;
    }
    html.push_str(&format!("<section class=\"work-publication\" data-publication-scope=\"{scope}\" data-publication-state=\"{:?}\"><p class=\"eyebrow\">{}</p>",answers.explanation_state,text(r.locale,"Work summary","작업 요약")));
    for index in 0..answers.prose.len() {
        reader_unit(html, r, answers, index);
    }
    explanation_diagnostic(html, r, answers);
    html.push_str("</section>");
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
    let subject = work
        .map(|w| format!("work:{}", w.work_item_id))
        .or_else(|| decision.map(|d| format!("decision:{}", d.decision.decision_id)))
        .unwrap_or_else(|| "repository".into());
    let answers = ReaderAnswers::new(
        answers,
        r.project_id,
        &subject,
        &r.requested_language,
        "article",
    );
    render_document_body(
        html, r, &answers, work, decision, records, related, snapshot,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_document_body(
    html: &mut String,
    r: &ViewerRequest,
    answers: &ReaderAnswers<'_>,
    work: Option<&UnderstandingWork>,
    decision: Option<&volicord_projections::UnderstandingDecision>,
    records: &str,
    related: &str,
    snapshot: bool,
) {
    let has_verification = work.is_some()
        || answers
            .sections
            .iter()
            .any(|s| s.layer == volicord_projections::ArticleLayer::Verification);
    let has_limits = answers.prose.iter().any(|a| a.question == "Limits")
        || answers.facts.iter().any(|a| {
            matches!(
                a.question.as_str(),
                "RecordedLimits" | "RecordedNonGoals" | "KnownLimit"
            )
        });
    let current = answers.provenance.is_some();
    let (back_view, back_id, back_label, kind) = if work.is_some() {
        (
            ViewerView::Work { work: None },
            "works",
            text(r.locale, "← Work list", "← 작업 목록"),
            "WORK DOCUMENT",
        )
    } else if decision.is_some() {
        (
            r.view.clone(),
            "works",
            text(r.locale, "← Work", "← 작업"),
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
    if current {
        if let Some(name) = answers
            .card_name()
            .filter(|name| Some(*name) != answers.reading_title())
        {
            html.push_str(&format!(
                "<p class=\"work-name\" data-publication-scope=\"name\" lang=\"{}\">{} · {}</p>",
                escape(&content_language_tag(&r.requested_language)),
                text(r.locale, "Work", "작업"),
                escape(name)
            ));
        }
    }
    if let Some((index, title)) = answers
        .prose
        .iter()
        .enumerate()
        .find(|(_, a)| a.question == "Heading")
    {
        html.push_str(&format!("<div id=\"{}\" tabindex=\"-1\"><h1 data-title-basis=\"generated-interpretation\" data-question=\"Heading\" lang=\"{}\" data-content-language=\"{}\">{}</h1></div>",reader_claim_anchor(answers,index),escape(&content_language_tag(&r.requested_language)),escape(&r.requested_language),escape(&title.text)));
        reader_citations(html, r, answers, index);
    } else if let Some(name) = answers
        .work_name
        .as_ref()
        .filter(|name| name.provenance.is_some())
    {
        let provenance = name.provenance.as_ref();
        html.push_str(&format!("<h1 data-title-basis=\"generated-interpretation\" data-publication-scope=\"name\" data-realization=\"{}\" lang=\"{}\">{}</h1>",provenance.map(|p| p.realization_fingerprint.as_str()).unwrap_or_default(),escape(&content_language_tag(&r.requested_language)),escape(name.reading_title().unwrap_or_default())));
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
        reader_citations(html, r, answers, index);
    }
    // The independent summary remains useful when detail is unavailable. In a
    // current article it is inspection material, not a second competing lead.
    if !current {
        if let Some(summary) = &answers.work_summary {
            render_card_publication(html, r, &answers.publication(summary, "summary"), "summary");
        }
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
    html.push_str(&format!("<details class=\"document-nav\"><summary>{}<span class=\"toc-cue\">{}</span></summary><nav aria-label=\"{}\"><ul class=\"toc-primary\">",
        text(r.locale,"TOC","목차"),text(r.locale,"On this page","이 문서의 내용"),text(r.locale,"Document sections","문서 목차")));
    for section in answers.sections.iter().filter(|s| {
        s.layer == volicord_projections::ArticleLayer::Article
            && s.unit_indices
                .iter()
                .any(|&i| !answers.prose[i].text.trim().is_empty())
    }) {
        if let Some(index) = section.heading_index {
            html.push_str(&format!(
                "<li><a href=\"#{}\">{}</a></li>",
                reader_claim_anchor(answers, index),
                escape(&answers.prose[index].text)
            ));
        }
    }
    html.push_str(&format!(
        "</ul><details class=\"toc-more\"><summary>{}</summary><ul>",
        text(r.locale, "Supporting details", "보조 자료")
    ));
    for (section, label) in [
        ("choice", text(r.locale, "Recorded choice", "기록된 선택")),
        (
            "verification",
            text(r.locale, "Verification report", "검증 보고"),
        ),
        ("limits", text(r.locale, "Limitations", "한계")),
        (
            "evidence-index",
            text(r.locale, "Evidence index", "근거 색인"),
        ),
        (
            "original",
            text(r.locale, "Original instructions", "원래 지시"),
        ),
        ("audit", text(r.locale, "Inspect records", "기록 검사")),
    ] {
        if (section == "original" && work.is_none())
            || (section == "verification" && !has_verification)
            || (section == "limits" && !has_limits)
            || (section == "choice" && decision.is_none())
        {
            continue;
        }
        html.push_str(&format!(
            "<li><a href=\"#{}\">{}</a></li>",
            document_anchor(answers, section),
            label
        ));
    }
    html.push_str(&format!(
        "</ul></details></nav></details><div id=\"{}\" tabindex=\"-1\">",
        document_anchor(answers, "reading")
    ));
    if !current {
        for (index, _) in answers
            .prose
            .iter()
            .enumerate()
            .filter(|(_, unit)| unit.question == "ExplanationAvailability")
        {
            reader_unit(html, r, answers, index);
        }
        if let Some(result) = work
            .and_then(|w| w.reading.answers.result.as_ref())
            .and_then(|r| r.original_text.as_ref())
        {
            html.push_str(&format!("<details class=\"recorded-reading\"><summary>{}</summary><blockquote lang=\"\">{}</blockquote></details>",
                text(r.locale,"Original recorded result · quotation", "원래 기록된 결과 · 인용"),escape(result)));
        }
    }
    render_article_layer(
        html,
        r,
        answers,
        volicord_projections::ArticleLayer::Article,
    );
    html.push_str("</div>");
    if let Some(d) = decision {
        render_recorded_choice(html, r, answers, d);
    }
    if let Some(work) = work {
        render_verification(html, r, answers, &work.reading.verification_report);
    } else if has_verification {
        html.push_str(&format!("<details class=\"reading-section verification-report\" id=\"{}\" tabindex=\"-1\"><summary>{}</summary>",document_anchor(answers,"verification"),text(r.locale,"Verification","검증")));
        render_article_layer(
            html,
            r,
            answers,
            volicord_projections::ArticleLayer::Verification,
        );
        html.push_str("</details>");
    }
    if has_limits {
        html.push_str(&format!("<section class=\"reading-section limits-section\" id=\"{}\" tabindex=\"-1\"><aside class=\"guard\"><h2>{}</h2>",document_anchor(answers,"limits"),answer_title("Limits",r.locale)));
        render_article_layer(html, r, answers, volicord_projections::ArticleLayer::Limits);
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
    render_evidence_index(html, r, answers);
    html.push_str("<div class=\"audit\">");
    if let Some(w) = work {
        html.push_str(&format!("<details class=\"record-box\" id=\"{}\"><summary>{}</summary><blockquote class=\"goal\" lang=\"\" data-statement-role=\"verified-canonical\">{}</blockquote><pre>{}</pre></details>",document_anchor(answers,"original"),text(r.locale,"Original instructions · complete Goal","원래 지시 · 전체 Goal"),escape(w.reading.goal.original_text.as_deref().unwrap_or(&w.title)),escape(&format!("{:?}",w.reading.goal.basis))));
    }
    html.push_str(&format!("<details class=\"record-box\" id=\"{}\"><summary>{}</summary><div class=\"record-facts\" data-statement-role=\"deterministic-derived\">",document_anchor(answers,"audit"),text(r.locale,"Full evidence · states · output identity","전체 근거 · 상태 · 출력 identity")));
    for fact in &answers.facts {
        html.push_str(&format!(
            "<p data-question=\"{}\">{}</p>",
            escape(&fact.question),
            escape(&fact.text)
        ));
    }
    html.push_str("</div>");
    for scoped in answers.work_name.iter().chain(answers.work_summary.iter()) {
        html.push_str(&format!(
            "<pre class=\"publication-provenance\">{}</pre>",
            escape(&format!("{:?}", scoped.provenance))
        ));
    }
    if current {
        if let Some(summary) = &answers.work_summary {
            render_card_publication(html, r, &answers.publication(summary, "summary"), "summary");
        }
    }
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
    }
    if let Some(w) = work {
        html.push_str(&format!(
            "<details id=\"{}\" data-verification-work=\"{}\"><summary>{}</summary><pre>{}</pre></details>",
            document_anchor(answers,"full-verification"),w.work_item_id,
            text(
                r.locale,
                "Complete verification read model (JSON)",
                "전체 검증 읽기 모델 (JSON)"
            ),
            escape(
                &serde_json::to_string_pretty(&w.reading.verification_report).unwrap_or_default()
            )
        ));
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
    html.push_str(records);
    html.push_str("</details></div></article>");
    // Every Source pane is a sibling of the reading article. On narrow
    // screens the article becomes inert while its selected pane is open.
    // This includes independently admitted summaries in the header or audit.
    if let Some(summary) = &answers.work_summary {
        render_source_excerpts(html, r, &answers.publication(summary, "summary"), snapshot);
    }
    render_source_excerpts(html, r, answers, snapshot);
}

fn render_evidence_index(html: &mut String, r: &ViewerRequest, answers: &ReaderAnswers<'_>) {
    html.push_str(&format!("<details class=\"reading-section evidence-index\" id=\"{}\" tabindex=\"-1\"><summary>{}</summary><p class=\"source-note\">{}</p>",
        document_anchor(answers,"evidence-index"),text(r.locale,"Evidence index", "근거 색인"),
        text(r.locale,"Start with the statement you want to check. Source ranges show observed bytes; original records show what was reported or chosen. Neither alone verifies every interpretation.",
            "확인할 문장에서 시작하세요. Source 범위는 관측된 바이트를, 원래 기록은 보고되거나 선택된 내용을 보여 줍니다. 어느 쪽도 그 자체로 모든 해석을 검증하지는 않습니다.")));
    if answers.evidence_index.is_empty() {
        html.push_str(&format!("<p>{}</p>",text(r.locale,"No admitted article claims are available to index. Original records remain inspectable.", "색인할 수 있는 유효한 설명 문장이 없습니다. 원래 기록은 검사할 수 있습니다.")));
    }
    for entry in &answers.evidence_index {
        let Some(unit) = answers.prose.get(entry.unit_index) else {
            continue;
        };
        let preview: String = unit.text.chars().take(160).collect();
        let section = answers.sections.iter().find(|s| {
            s.heading_index == Some(entry.unit_index) || s.unit_indices.contains(&entry.unit_index)
        });
        let section_name = section
            .and_then(|s| s.heading_index)
            .map(|i| answers.prose[i].text.as_str())
            .unwrap_or_else(|| match section.map(|s| s.layer) {
                Some(volicord_projections::ArticleLayer::Verification) => {
                    text(r.locale, "Verification", "검증")
                }
                Some(volicord_projections::ArticleLayer::Limits) => {
                    text(r.locale, "Limitations", "한계")
                }
                _ => text(r.locale, "Article statement", "설명 문장"),
            });
        html.push_str(&format!("<details class=\"record-box evidence-entry\"><summary><span><small>{}</small><span>{}{}</span></span></summary><a class=\"button\" href=\"#{}\">{}</a>",
            escape(section_name),escape(&preview),if preview.len()<unit.text.len(){"…"}else{""},
            reader_claim_anchor(answers,entry.unit_index),text(r.locale,"Read this statement in context", "문맥에서 이 문장 읽기")));
        if !entry.citations.is_empty() {
            reader_citations(html, r, answers, entry.unit_index);
        }
        if let Some(provenance) = &answers.provenance {
            html.push_str("<ul class=\"evidence-records\">");
            for evidence in provenance
                .evidence
                .iter()
                .filter(|e| entry.evidence_keys.contains(&e.key))
            {
                let label = match evidence.record_kind.as_str() {
                    "context_item" => {
                        text(r.locale, "Original context / request", "원래 맥락 / 요청")
                    }
                    "checkpoint" => text(r.locale, "Recorded work observation", "기록된 작업 관측"),
                    "decision" => text(
                        r.locale,
                        "Recorded choice or recommendation",
                        "기록된 선택·권고",
                    ),
                    "question" => text(
                        r.locale,
                        "Recorded question and alternatives",
                        "기록된 질문·대안",
                    ),
                    _ => text(r.locale, "Prepared supporting record", "준비된 근거 기록"),
                };
                html.push_str(&format!("<li><a href=\"#{}\">{} · {}</a><details><summary>{}</summary><p>{} {}@{} · {}</p></details></li>",
                    document_anchor(answers,"audit"),label,escape(&evidence.field),text(r.locale,"Exact reference", "정확한 참조"),escape(&evidence.record_kind),escape(&evidence.identity),evidence.revision,escape(&evidence.key)));
            }
            html.push_str("</ul>");
        }
        html.push_str("</details>");
    }
    html.push_str("</details>");
}

#[path = "reading_verification.rs"]
mod verification;
use verification::render_verification;

fn repository_boundaries(
    html: &mut String,
    r: &ViewerRequest,
    answers: &ReaderAnswers<'_>,
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
        let summary = &basis.content["repository"][key];
        let count = summary["area_count"].as_u64().map_or_else(
            || text(r.locale, "unknown", "미확인").into(),
            |v| v.to_string(),
        );
        html.push_str(&format!(
            "<p>{}: {}</p>",
            text(r.locale, "Recorded boundaries", "기록된 경계"),
            escape(&count)
        ));
    }
    html.push_str(&format!("<p>{}</p><code>{}</code>",text(r.locale,
        "Full inventory and coverage path lists are not embedded. Exact snapshot-bound details remain available outside Viewer:",
        "전체 inventory와 coverage 경로 목록은 포함하지 않습니다. 정확한 snapshot의 상세 정보는 Viewer 밖에서 확인할 수 있습니다:"),
        escape(basis.content["scope_detail"]["command"].as_str().unwrap_or("repository explain evidence"))));
    html.push_str("</details>");
}

fn reading_warning(
    html: &mut String,
    _r: &ViewerRequest,
    kind: &str,
    severity: &str,
    title: &str,
    message: &str,
) {
    if kind == "source" || severity == "history" {
        html.push_str(&format!("<details class=\"reading-warning\" data-warning=\"{}\" data-severity=\"{}\"><summary>{}</summary><p>{}</p></details>", escape(kind),escape(severity),escape(title),escape(message)));
    } else {
        html.push_str(&format!("<aside class=\"reading-warning\" role=\"note\" aria-label=\"{}\" data-warning=\"{}\" data-severity=\"{}\"><strong>{}</strong><p>{}</p></aside>", escape(title),escape(kind),escape(severity),escape(title),escape(message)));
    }
}

fn opening_context(
    html: &mut String,
    r: &ViewerRequest,
    answers: &ReaderAnswers<'_>,
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
    use volicord_projections::ReadingCautionKind as Kind;
    let failed: Vec<_> = answers
        .cautions
        .iter()
        .filter(|c| c.kind == Kind::FailedCheck)
        .collect();
    let historical = |c: &&volicord_projections::ReadingCaution| {
        work.is_some_and(|w| {
            c.checkpoint.as_ref().is_some_and(|reference| {
                w.reading.verification_report.checks.iter().any(|check| {
                    &check.original == reference
                        && verification::historical_check(check, &w.reading.verification_report)
                })
            })
        })
    };
    if failed.iter().any(|c| !historical(c)) {
        reading_warning(html, r, "failure", "danger", text(r.locale,"Unresolved verification failure", "해결 여부가 확인되지 않은 검증 실패"),
            text(r.locale,"A failed report remains unresolved. A later unrelated pass does not establish a retest or current coverage.","실패 보고의 해결 여부가 확인되지 않았습니다. 무관한 후속 통과는 재검증이나 현재 coverage를 확정하지 않습니다."));
        if let Some(w) = work {
            html.push_str("<p class=\"failure-check-links\">");
            for (index, check) in w
                .reading
                .verification_report
                .checks
                .iter()
                .enumerate()
                .filter(|(_, c)| {
                    c.reported_state == volicord_projections::ReportedCheckState::Failed
                        && !verification::historical_check(c, &w.reading.verification_report)
                })
                .take(3)
            {
                let target = if index < 64 {
                    verification::check_anchor(answers, check)
                } else {
                    document_anchor(answers, "full-verification")
                };
                html.push_str(&format!(
                    "<a href=\"#{target}\">{} {}</a> ",
                    text(r.locale, "Inspect failed report", "실패 보고 확인"),
                    index + 1
                ));
            }
            if failed.len() > 3 {
                html.push_str(&format!(
                    "<a href=\"#{}\">{}</a>",
                    document_anchor(answers, "full-verification"),
                    text(r.locale, "All original reports", "전체 원래 보고")
                ));
            }
            html.push_str("</p>");
        }
    }
    if failed.iter().any(historical)
        && !answers
            .facts
            .iter()
            .any(|f| f.question == "HistoricalAdversity")
    {
        reading_warning(html, r, "historical", "history", text(r.locale,"Historical failure", "과거 검증 실패"),
            text(r.locale,"An earlier failure remains recorded. Historical applicability or an explicitly compared rerun establishes no whole-work coverage.","이전 실패는 원래 보고에 남아 있습니다. 과거 적용 범위나 명시적으로 비교한 재실행이 작업 전체의 coverage를 확정하지는 않습니다."));
    }
    if answers
        .cautions
        .iter()
        .any(|c| c.kind == Kind::PartialCheck && !c.historical)
    {
        reading_warning(html, r,"partial","caution",text(r.locale,"Partial verification", "부분 검증"),
            text(r.locale,"The latest report covers only part of the intended verification. Inspect the original report for its stated limits.","최근 보고는 의도한 검증의 일부만 다룹니다. 원래 보고에서 명시된 한계를 확인하세요."));
    } else if work.is_none()
        && failed.is_empty()
        && answers.cautions.iter().any(|c| {
            matches!(
                c.kind,
                Kind::NotRun | Kind::CoverageUnknown | Kind::HistoricalCoverage
            )
        })
    {
        reading_warning(html,r,"unverified","note",text(r.locale,"Coverage remains unestablished", "검증 범위 미확인"),
            text(r.locale,"A reported pass or process exit does not establish current assertion coverage. Skips, historical results and unresolved comparison remain separate in the report.","보고된 통과나 프로세스 종료만으로 현재 assertion coverage를 확정하지 않습니다. 미실행·과거 결과·비교 미해결은 보고에서 구분됩니다."));
    }
    let rejected = work
        .and_then(|w| w.reading.answers.acceptance.as_ref())
        .is_some_and(|s| s.user_acceptance.state == UserAcceptanceState::Rejected);
    for fact in answers.facts.iter().filter(|a| {
        matches!(
            a.question.as_str(),
            "HistoricalAdversity"
                | "VerificationCoverage"
                | "SourceEvidenceGap"
                | "Conflicts"
                | "ReviewBasis"
        ) || (rejected && a.question == "UserAcceptance")
    }) {
        // Detailed verification already carries these facts; avoid duplicate notices.
        if fact.question == "VerificationCoverage" {
            continue;
        }
        let (kind, severity, title) = match fact.question.as_str() {
            "HistoricalAdversity" => (
                "history",
                "history",
                text(r.locale, "Prior adverse observations", "과거 문제 관측"),
            ),
            "SourceEvidenceGap" => (
                "source",
                "note",
                text(
                    r.locale,
                    "Supporting record Source gap",
                    "원래 기록의 Source 한계",
                ),
            ),
            "UserAcceptance" => (
                "rejected",
                "danger",
                text(
                    r.locale,
                    "Latest recorded acceptance: rejected",
                    "최근 기록된 수락: 거부",
                ),
            ),
            _ => (
                "review",
                "caution",
                text(
                    r.locale,
                    "Applicability needs review",
                    "적용 가능성 검토 필요",
                ),
            ),
        };
        reading_warning(html, r, kind, severity, title, &fact.text);
    }
    if let Some(w) = work {
        verification::render_digest(html, r, answers, &w.reading.verification_report);
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
}

fn explanation_diagnostic(html: &mut String, r: &ViewerRequest, answers: &ReaderAnswers<'_>) {
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
            "<div class=\"reading-warning explanation-diagnostic\" role=\"note\" data-warning=\"source\" data-severity=\"caution\"><p class=\"dependency-reason\"><strong>{}:</strong> {}</p><details><summary>{}</summary><p>{}</p></details></div>",
            text(r.locale, "Explanation limitation", "설명 한계"), reason,
            text(r.locale, "Exact dependency diagnostic", "정확한 의존 근거 진단"), escape(diagnostic)
        ));
    }
}

fn source_excerpt_anchor(
    answers: &ReaderAnswers<'_>,
    c: &volicord_projections::ExplanationCitation,
) -> String {
    let key = serde_json::json!([c.source_id, c.field, c.start_byte, c.end_byte, c.use_kind]);
    format!(
        "reader-{}-source-{}",
        answers.namespace,
        volicord_projections::explanation_text_hash(&key.to_string()).trim_start_matches("sha256:")
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
    answers: &ReaderAnswers<'_>,
    snapshot: bool,
) {
    let Some(provenance) = &answers.provenance else {
        return;
    };
    if provenance.citations.is_empty() {
        return;
    }
    html.push_str("<aside class=\"appendix\" data-reading-role=\"cited-source\" data-statement-role=\"observed-fact\">");
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
            let mut context = String::new();
            if let Some(e) = excerpt {
                if c.use_kind == volicord_projections::ExplanationSourceUse::CurrentCode {
                    let comparison=match e.comparison.as_deref() {
                        Some("changed")=>text(r.locale,"The explicit before/after file observations differ. Selected ranges are not a diff.","명시적 이전/이후 파일 관측이 다릅니다. 선택된 범위는 diff가 아닙니다."),
                        Some("no_change")=>text(r.locale,"The paired file observations have identical bytes. No change is shown.","짝지어진 파일 관측의 바이트가 동일합니다. 변경이 표시되지 않습니다."),
                        _=>text(r.locale,"Before-state was not retained. A before/after comparison is unavailable.","이전 상태가 보존되지 않았습니다. 이전/이후 비교를 이용할 수 없습니다."),
                    };
                    context.push_str(&format!("<p class=\"source-note\">{}</p>", comparison));
                    if let Some(before_index) = e.before_source_id.as_ref().and_then(|id| {
                        answer.citations.iter().position(|b| {
                            &b.source_id == id
                                && b.use_kind
                                    == volicord_projections::ExplanationSourceUse::BeforeCode
                        })
                    }) {
                        context.push_str(&format!(
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
                    context.push_str(&format!("<p class=\"source-note\">{}</p>",text(r.locale,"Bounded file observation: surrounding code was not retained. Coordinates refer to the repository file; missing context remains unobserved.","제한된 파일 관측: 주변 코드는 보존되지 않았습니다. 좌표는 저장소 파일 기준이며 누락된 맥락은 관측되지 않았습니다.")));
                }
                let syntax = crate::source_highlight::language(
                    c,
                    e.source_representation.as_deref(),
                    e.execution.is_some(),
                );
                if e.text.contains('\0') {
                    html.push_str(&format!("<p class=\"source-note\">{}</p>",text(r.locale,"This excerpt contains NUL characters. HTML parsing displays replacement characters for NUL; exact NUL text is available only in the permitted live citation response.","이 발췌에는 NUL 문자가 있습니다. HTML 파서는 NUL을 대체 문자로 표시합니다. 정확한 NUL 텍스트는 허용된 실시간 인용 응답에서만 읽을 수 있습니다.")));
                }
                html.push_str(&format!(
                    "<p class=\"code-caption\">{} · {}</p>",
                    syntax.map(|s| s.label()).unwrap_or(text(
                        r.locale,
                        "Plain text",
                        "일반 텍스트"
                    )),
                    text(
                        r.locale,
                        "Observed cited range · syntax colors are a reading aid",
                        "관측된 인용 범위 · 구문 색상은 읽기 보조 표시"
                    )
                ));
                html.push_str(&format!("<pre class=\"code-region\" tabindex=\"0\" role=\"region\" aria-label=\"{}: {}:{}–{}\"><span class=\"source-line-numbers\" aria-hidden=\"true\">{}</span><code lang=\"\" data-syntax=\"{}\">{}</code></pre>",text(r.locale,"Original Source","Source 원문"),escape(&c.locator),c.start_line,e.displayed_end_line,e.text.split_inclusive('\n').enumerate().map(|(line,_)|format!("<span data-line=\"{}\"></span>",c.start_line+line)).collect::<String>(),syntax.map(|s|s.key()).unwrap_or("plain"),crate::source_highlight::html(&e.text,syntax)));
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
            context.push_str(&format!("<p class=\"source-note\">{}</p>",text(r.locale,
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
            html.push_str(&format!("<details class=\"source-context\"><summary>{}</summary>{context}<p class=\"source-note\">{}: {} / 16,384 UTF-8 bytes · 4,096 / {}</p></details>",
                text(r.locale,"Observation scope and limits", "관측 범위와 한계"),text(r.locale,"Included unique cited text", "포함된 고유 인용 텍스트"),
                answers.source_excerpts.iter().map(|e|e.text.len()).sum::<usize>(),text(r.locale,"range maximum", "범위별 최대")));
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
    let mut seen_decisions = std::collections::BTreeSet::new();
    let decisions = p
        .selected_work_decisions
        .iter()
        .chain(p.decision_catalog.iter().filter(|_| snapshot))
        .filter(|d| w.decision_ids.contains(&d.decision.decision_id))
        .filter(|d| seen_decisions.insert(d.decision.decision_id))
        .collect::<Vec<_>>();
    if decisions.len() < w.decision_ids.len() {
        empty_state(&mut records,text(r.locale,
            "Some related Decision records are outside this publication's bounded selection. Use the exact Work in the live reader or CLI/MCP to inspect them.",
            "일부 관련 Decision은 이 문서의 제한된 선택 범위 밖에 있습니다. live 독자 또는 CLI/MCP에서 정확한 Work로 확인하세요."));
    }
    let mut related = String::new();
    let mut decision_documents = String::new();
    for d in decisions
        .iter()
        .filter(|d| w.decision_ids.contains(&d.decision.decision_id))
    {
        let mut document = String::new();
        decision_detail(&mut document, r, d, p, snapshot);
        // A Decision may actually apply to several Works. Scope its DOM anchors
        // to this reading context while retaining the canonical/citation identity.
        let ids = document
            .split(" id=\"")
            .skip(1)
            .filter_map(|s| s.split('"').next())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        for id in ids {
            let scoped = format!("work-{}-{id}", w.work_item_id);
            document = document
                .replace(&format!("id=\"{id}\""), &format!("id=\"{scoped}\""))
                .replace(&format!("href=\"#{id}\""), &format!("href=\"#{scoped}\""))
                .replace(
                    &format!("data-origin=\"{id}\""),
                    &format!("data-origin=\"{scoped}\""),
                );
        }
        related.push_str(&format!(
            "<p><a href=\"#work-{}-decision-{}\">{}</a></p>",
            w.work_item_id,
            d.decision.decision_id,
            escape(&decision_choice_attribution(&d.decision, r.locale))
        ));
        decision_documents.push_str(&document);
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
    html.push_str(&decision_documents);
    html.push_str("</section>");
}

fn render_recorded_choice(
    html: &mut String,
    r: &ViewerRequest,
    answers: &ReaderAnswers<'_>,
    d: &volicord_projections::UnderstandingDecision,
) {
    html.push_str(&format!("<section class=\"reading-section recorded-choice\" id=\"{}\" tabindex=\"-1\" data-statement-role=\"verified-canonical\">", document_anchor(answers,"choice")));
    heading(
        html,
        2,
        text(
            r.locale,
            "Recorded choice and reasons",
            "기록된 선택과 이유",
        ),
    );
    if let Some(question) = &d.decision.question_context {
        heading(html, 3, text(r.locale, "The question", "질문"));
        html.push_str(&format!(
            "<blockquote lang=\"\">{}</blockquote>",
            escape(&question.prompt)
        ));
    }
    heading(
        html,
        3,
        text(
            r.locale,
            "Alternatives · expected consequences",
            "대안 · 예상 결과",
        ),
    );
    html.push_str("<dl class=\"choice-alternatives\">");
    for alternative in &d.decision.displayed_alternatives {
        definition(html, &alternative.label, &alternative.consequence);
    }
    html.push_str("</dl>");
    heading(
        html,
        3,
        text(
            r.locale,
            "Recorded agent recommendation",
            "기록된 에이전트 권고",
        ),
    );
    if let Some(fact) =
        volicord_projections::decision_answers(&d.decision, &r.requested_language, r.locale.fixed())
            .facts
            .iter()
            .find(|f| f.question == "RecommendedChoice")
    {
        html.push_str(&format!("<p>{}</p>", escape(&fact.text)));
    }
    if let Some(reason) = &d.reading.recommendation_rationale.original_text {
        html.push_str(&format!(
            "<blockquote lang=\"\">{}</blockquote>",
            escape(reason)
        ));
    }
    heading(html, 3, text(r.locale, "User choice", "사용자 선택"));
    html.push_str(&format!(
        "<p>{}</p>",
        escape(&decision_choice_attribution(&d.decision, r.locale))
    ));
    if let Some(reason) = &d.reading.user_rationale.original_text {
        heading(
            html,
            3,
            text(
                r.locale,
                "Recorded user rationale · quotation",
                "기록된 사용자 이유 · 인용",
            ),
        );
        html.push_str(&format!(
            "<blockquote lang=\"\">{}</blockquote>",
            escape(reason)
        ));
    } else {
        html.push_str(&format!(
            "<p class=\"source-note\">{}</p>",
            text(
                r.locale,
                "User rationale is not recorded; agent recommendation does not supply it.",
                "사용자 근거가 기록되지 않았습니다. 에이전트 권고는 이를 대신하지 않습니다."
            )
        ));
    }
    html.push_str(&format!("<p class=\"scope-summary\">{}</p><p class=\"source-note\">{}</p></section>",
        escape(decision_scope_label(d.decision.work_scope, r.locale)),
        text(r.locale, "These are retained statements and expected consequences, not verified implementation effects. Source availability is assessed separately.",
            "보존된 진술과 예상 결과이며 검증된 구현 효과가 아닙니다. Source 가용성은 별도로 확인합니다.")));
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
        ViewerView::RepositoryExplanation => repository_explanation(html, r, p, false),
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
    repository_explanation(html, r, &p.for_repository_reading(), true);
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
        pagination(html, r, p.work_count, r.view.detail().work_page);
    }
    html.push_str("</section>");
}

fn pagination(html: &mut String, r: &ViewerRequest, count: usize, page: usize) {
    html.push_str("<nav aria-label=\"List pages\">");
    for next in [
        page.checked_sub(1),
        page.checked_add(1)
            .filter(|next| next.saturating_mul(64) < count),
    ]
    .into_iter()
    .flatten()
    {
        link(
            html,
            r,
            ViewerView::WorkPage { page: next },
            &format!("{} {}", text(r.locale, "Page", "페이지"), next + 1),
            None,
        );
    }
    html.push_str("</nav>");
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
