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
pub(super) fn navigation(html: &mut String, request: &ViewerRequest, snapshot: bool) {
    html.push_str("<nav aria-label=\"Viewer\"><ul class=\"view-nav\">");
    for (view, label, id) in [
        (
            ViewerView::Overview,
            text(request.locale, "Overview", "개요"),
            "overview",
        ),
        (
            ViewerView::Work { work: None },
            text(request.locale, "Work", "작업"),
            "works",
        ),
        (
            ViewerView::Code {
                scope: match request.view.selection() {
                    volicord_projections::WorkSelector::ExactWork(id) => CodeScope::Work(Some(id)),
                    _ => CodeScope::Repository,
                },
                entity: None,
            },
            text(request.locale, "Code Understanding", "코드 이해"),
            "code",
        ),
        (
            ViewerView::Decisions { decision: None },
            text(request.locale, "Decisions", "결정"),
            "decision-reading",
        ),
        (
            ViewerView::Tools {
                tool: ViewerTool::Status,
            },
            text(request.locale, "Analysis", "분석"),
            "health",
        ),
        (
            ViewerView::Tools {
                tool: ViewerTool::Documents,
            },
            text(request.locale, "Tools", "도구"),
            "documents",
        ),
    ] {
        html.push_str("<li>");
        let selected = !snapshot
            && match (&request.view, &view) {
                (ViewerView::Tools { tool }, ViewerView::Tools { tool: target }) => {
                    (*tool == ViewerTool::Status) == (*target == ViewerTool::Status)
                }
                _ => request.view.key() == view.key(),
            };
        let start = html.len();
        link(html, request, view, label, snapshot.then_some(id));
        if selected {
            html.insert_str(start + 2, " aria-current=\"page\"");
        }
        html.push_str("</li>");
    }
    html.push_str("</ul></nav>");
    if !snapshot && matches!(request.view, ViewerView::Tools { .. }) {
        html.push_str("<nav aria-label=\"Tools\"><ul class=\"view-nav\">");
        for (tool, label) in [
            (
                ViewerTool::Documents,
                text(request.locale, "Documents", "문서"),
            ),
            (ViewerTool::Memory, text(request.locale, "Memory", "기억")),
            (
                ViewerTool::Status,
                text(request.locale, "Analysis and runtime", "분석 및 런타임"),
            ),
            (
                ViewerTool::Evidence,
                text(request.locale, "Evidence", "근거"),
            ),
        ] {
            html.push_str("<li>");
            link(html, request, ViewerView::Tools { tool }, label, None);
            html.push_str("</li>");
        }
        html.push_str("</ul></nav>");
    }
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
    request: &ViewerRequest,
    p: &ProjectProjection,
    u: &ProjectUnderstanding,
    snapshot: bool,
) {
    section_start(
        html,
        "overview",
        text(request.locale, "Project Understanding", "프로젝트 이해"),
    );
    heading(html, 3, text(request.locale, "Purpose", "목적"));
    if u.project_purpose.is_empty() {
        empty_state(
            html,
            text(
                request.locale,
                "No source-grounded Project purpose is recorded.",
                "source-grounded 프로젝트 목적이 기록되지 않았습니다.",
            ),
        );
    }
    for purpose in &u.project_purpose {
        html.push_str("<p data-statement-role=\"verified-canonical\">");
        html.push_str(&escape(&purpose.statement));
        html.push_str("</p>");
    }
    for (label, works) in [
        (
            text(request.locale, "Current Work", "현재 작업"),
            &u.work_overview.current,
        ),
        (
            text(request.locale, "Recent outcomes", "최근 결과"),
            &u.work_overview.completed,
        ),
        (
            text(request.locale, "Remaining Work", "남은 작업"),
            &u.work_overview.remaining,
        ),
    ] {
        heading(html, 3, label);
        if works.items.is_empty() && works.complete {
            empty_state(
                html,
                text(
                    request.locale,
                    "No recorded Work in this section.",
                    "이 항목에 기록된 작업이 없습니다.",
                ),
            );
        }
        html.push_str("<details class=\"category-details\"><summary>");
        html.push_str(text(
            request.locale,
            "Work counts and omissions",
            "작업 수 및 생략",
        ));
        html.push_str("</summary>");
        html.push_str(&format!(
            "<p class=\"category-count\">{}: {} · {}: {} · {}: {}</p>",
            text(request.locale, "Total", "전체"),
            if works.complete {
                works.total.to_string()
            } else {
                text(request.locale, "unknown", "알 수 없음").into()
            },
            text(request.locale, "Displayed", "표시"),
            works.items.len(),
            text(request.locale, "Omitted", "생략"),
            if works.complete {
                works.omitted.to_string()
            } else {
                text(request.locale, "unknown", "알 수 없음").into()
            }
        ));
        html.push_str("</details><div class=\"work-list\">");
        for work in &works.items {
            work_summary(html, request, p, work, snapshot);
        }
        html.push_str("</div>");
        if works.omitted > 0 {
            empty_state(
                html,
                text(
                    request.locale,
                    "Additional Works are available; open Work navigation.",
                    "추가 작업이 있습니다. 작업 탐색을 여세요.",
                ),
            );
        }
    }
    if !u.open_questions.is_empty()
        || !u.risks_assumptions_and_limits.is_empty()
        || !u.unresolved_work_grouping.is_empty()
    {
        heading(
            html,
            3,
            text(
                request.locale,
                "Blockers and uncertainty",
                "차단 문제 및 불확실성",
            ),
        );
        for question in &u.open_questions {
            empty_state(html, &question.prompt);
        }
        for risk in &u.risks_assumptions_and_limits {
            empty_state(html, &risk.statement);
        }
        for unresolved in &u.unresolved_work_grouping {
            empty_state(html, &unresolved.reason);
        }
    }
    contextual_limits(html, request, p, snapshot);
    heading(
        html,
        3,
        text(request.locale, "Recorded next steps", "기록된 다음 단계"),
    );
    for work in &u.work_overview.next_steps.items {
        let answers = volicord_projections::work_answers(
            work,
            &request.requested_language,
            request.locale.fixed(),
        );
        if let Some(answer) = answers.next_step_answer() {
            html.push_str(&format!(
                "<p class=\"next-action\">{}</p>",
                escape(&answer.text)
            ));
        }
    }
    if u.work_overview.next_steps.total == 0 && u.work_overview.next_steps.complete {
        empty_state(
            html,
            text(
                request.locale,
                "Next step unavailable: no Checkpoint records one.",
                "다음 단계 없음: 기록한 체크포인트가 없습니다.",
            ),
        );
    }
    section_end(html);
}
fn work_summary(
    html: &mut String,
    r: &ViewerRequest,
    p: &ProjectProjection,
    w: &UnderstandingWork,
    snapshot: bool,
) {
    let answers = volicord_projections::work_answers(w, &r.requested_language, r.locale.fixed());
    html.push_str(&format!("<article class=\"understanding-card work-item work-summary\" data-work-id=\"{}\" data-work-state=\"{}\"><h4>", w.work_item_id, understanding_work_state_key(w.state)));
    link(
        html,
        r,
        ViewerView::Work {
            work: Some(w.work_item_id),
        },
        work_reading_display(&w.reading.goal, r.locale),
        snapshot
            .then(|| format!("work-{}", w.work_item_id))
            .as_deref(),
    );
    html.push_str("</h4>");
    html.push_str(&format!(
        "<p class=\"work-state\"><span class=\"badge\">{}</span></p>",
        escape(understanding_work_state_label(w.state, r.locale))
    ));
    for (label, question) in [
        (text(r.locale, "Problem / goal", "문제 / 목표"), "Purpose"),
        (text(r.locale, "Outcome", "결과"), "ReportedChange"),
        (text(r.locale, "Verification", "검증"), "Verification"),
    ] {
        if let Some(answer) = answers.prose.iter().find(|a| a.question == question) {
            html.push_str(&format!(
                "<p data-question=\"{}\"><strong>{}:</strong> {}</p>",
                question,
                label,
                escape(&answer.text)
            ));
        } else if question == "ReportedChange" {
            empty_state(html, text(r.locale, "Outcome explanation is not available. Open Work for recorded facts and evidence.", "결과 설명이 없습니다. 작업을 열어 기록된 사실과 근거를 확인하세요."));
        }
    }
    // Verification failure/historical coverage and actual direction are ordinary facts,
    // even when an interpretation is absent or offers a different next step.
    for fact in answers.facts.iter().filter(|a| {
        matches!(
            a.question.as_str(),
            "VerificationState"
                | "UserReview"
                | "UserAcceptance"
                | "HistoricalAdversity"
                | "VerificationCoverage"
                | "SourceEvidenceGap"
                | "ReportedResult"
                | "RecordedLimits"
        )
    }) {
        html.push_str(&format!(
            "<p data-question=\"{}\">{}</p>",
            escape(&fact.question),
            escape(&fact.text)
        ));
    }
    if let Some(action) = answers.next_step_answer() {
        html.push_str(&format!(
            "<p class=\"next-action\" data-question=\"{}\">{}</p>",
            escape(&action.question),
            escape(&action.text)
        ));
    }
    for candidate in related_learning(p, w.work_item_id) {
        html.push_str("<p class=\"learning-inspection-reference\">");
        link(
            html,
            r,
            ViewerView::Work {
                work: Some(w.work_item_id),
            },
            text(
                r.locale,
                "Inspect related Learning (Session Candidate)",
                "관련 학습 확인 (Session Candidate)",
            ),
            snapshot
                .then(|| format!("learning-{}", candidate.candidate_id))
                .as_deref(),
        );
        html.push_str("</p>");
    }
    html.push_str("</article>");
}
fn answer_title<'a>(question: &str, locale: ViewerLocale) -> &'a str {
    match question {
        "Purpose" => text(locale, "Goal / problem", "목표 / 문제"),
        "ReportedChange" => text(locale, "Change / investigation", "변경 / 조사"),
        "ExpectedEffect" => text(locale, "Result / expected effect", "결과 / 기대 효과"),
        "Verification" => text(locale, "Verification", "검증"),
        "NextStep" => text(locale, "Next step / uncertainty", "다음 단계 / 불확실성"),
        "UserRationale" => text(locale, "User rationale", "사용자 이유"),
        "Recommendation" => text(locale, "Agent recommendation basis", "에이전트 권고 근거"),
        "Consequences" => text(locale, "Alternatives / trade-offs", "대안 / 절충"),
        "Applicability" => text(locale, "Applicability", "적용 범위"),
        "Limits" => text(locale, "Remaining uncertainty", "남은 불확실성"),
        _ => text(locale, "Explanation availability", "설명 가용성"),
    }
}
pub(super) fn render_answers(
    html: &mut String,
    r: &ViewerRequest,
    answers: &volicord_projections::QuestionAnswers,
    compact: bool,
) {
    let current = answers.provenance.is_some();
    html.push_str(&format!(
        "<div class=\"{}\" data-statement-role=\"{}\">",
        if current {
            "work-explanation"
        } else {
            "answer-unavailable"
        },
        if current {
            "generated-interpretation"
        } else {
            "deterministic-derived"
        }
    ));
    if current {
        empty_state(html,text(r.locale,"Host interpretation; claims remain grounded reports, not independent verification.","호스트 해석입니다. 근거 있는 보고이며 독립 검증을 뜻하지 않습니다."));
    }
    for answer in &answers.prose {
        if compact
            && !matches!(
                answer.question.as_str(),
                "Purpose"
                    | "ReportedChange"
                    | "Verification"
                    | "NextStep"
                    | "Limits"
                    | "ExplanationAvailability"
            )
        {
            continue;
        }
        if !compact {
            html.push_str("<div class=\"answer-section\">");
            heading(html, 4, answer_title(&answer.question, r.locale));
        }
        html.push_str(&format!(
            "<p data-question=\"{}\">{}</p>",
            escape(&answer.question),
            escape(&answer.text)
        ));
        if !compact {
            html.push_str("</div>");
        }
    }
    if let Some(provenance) = &answers.provenance {
        html.push_str(&format!(
            "<details class=\"explanation-grounding\"><summary>{}</summary><pre>{}</pre></details>",
            text(
                r.locale,
                "Explanation evidence and generator",
                "설명 근거 및 생성자"
            ),
            escape(&format!("{provenance:?}"))
        ));
    }
    if answers.diagnostic.is_some() {
        html.push_str(&format!(
            "<details><summary>{}</summary><p>{}</p></details>",
            text(r.locale, "Explanation diagnostic", "설명 진단"),
            escape(answers.diagnostic.as_deref().unwrap_or_default())
        ));
    }
    html.push_str(
        "</div><div class=\"fact-states\" data-statement-role=\"deterministic-derived\">",
    );
    for fact in &answers.facts {
        if matches!(
            fact.question.as_str(),
            "UserRationale" | "RecommendedChoice" | "DeclaredScope" | "ReviewBasis"
        ) {
            html.push_str("<div class=\"decision-fact\">");
            heading(
                html,
                4,
                match fact.question.as_str() {
                    "UserRationale" => text(r.locale, "User rationale", "사용자 이유"),
                    "RecommendedChoice" => text(r.locale, "Recorded recommendation", "기록된 권고"),
                    "ReviewBasis" => text(
                        r.locale,
                        "Applicability needs review",
                        "적용 가능성 검토 필요",
                    ),
                    _ => text(r.locale, "Declared scope", "선언된 범위"),
                },
            );
        }
        html.push_str(&format!(
            "<p data-question=\"{}\">{}</p>",
            escape(&fact.question),
            escape(&fact.text)
        ));
        if matches!(
            fact.question.as_str(),
            "UserRationale" | "RecommendedChoice" | "DeclaredScope" | "ReviewBasis"
        ) {
            html.push_str("</div>");
        }
    }
    html.push_str("</div>");
}
pub(super) fn work_explanation(
    html: &mut String,
    r: &ViewerRequest,
    w: &UnderstandingWork,
    compact: bool,
) {
    let answers = volicord_projections::work_answers(w, &r.requested_language, r.locale.fixed());
    render_answers(html, r, &answers, compact);
    // Quotation has an explicit evidence role; never the ordinary explanation.
    if let Some(result) = &w.reading.answers.result {
        html.push_str("<details class=\"result-evidence\"><summary>");
        html.push_str(text(
            r.locale,
            "Reported result quotation",
            "보고된 결과 인용",
        ));
        html.push_str("</summary><p>");
        html.push_str(&escape(result.original_text.as_deref().unwrap_or("")));
        html.push_str("</p><pre>");
        html.push_str(&escape(&format!("{:?}", result.basis)));
        html.push_str("</pre></details>");
    }
}

fn work_detail(
    html: &mut String,
    r: &ViewerRequest,
    p: &ProjectProjection,
    w: &UnderstandingWork,
    snapshot: bool,
) {
    section_start(
        html,
        &format!("work-{}", w.work_item_id),
        text(r.locale, "Work detail", "작업 상세"),
    );
    html.push_str("<p class=\"selection-label\">");
    html.push_str(text(r.locale, "Selected Work", "선택한 작업"));
    html.push_str("</p>");
    let decisions = if snapshot {
        p.decision_catalog
            .iter()
            .filter(|d| w.decision_ids.contains(&d.decision.decision_id))
            .cloned()
            .collect::<Vec<_>>()
    } else {
        p.selected_work_decisions.clone()
    };
    render_work_card(html, r, w, &decisions);
    if !snapshot || p.selection.work_item_id == Some(w.work_item_id) {
        contextual_limits(html, r, p, snapshot);
    }

    html.push_str(
        "<details class=\"work-state-history\" data-reading-role=\"audit-history\"><summary>",
    );
    html.push_str(text(
        r.locale,
        "Verification and original state observations",
        "검증 및 원래 상태 관찰",
    ));
    html.push_str("</summary>");
    for state in &w.reading.states {
        html.push_str(&format!("<p>{}</p>", escape(&format!("{state:?}"))));
    }
    html.push_str("</details>");
    heading(
        html,
        3,
        text(r.locale, "Decisions and Questions", "결정 및 질문"),
    );
    for decision in &decisions {
        if !snapshot {
            decision_link(html, r, decision, false);
        } else {
            empty_state(
                html,
                &decision_choice_attribution(&decision.decision, r.locale),
            );
        }
    }
    heading(
        html,
        3,
        text(
            r.locale,
            "Related Learning — Candidate Inspection",
            "관련 학습 — Candidate Inspection",
        ),
    );
    if p.candidate_dependency != volicord_projections::CandidateDependencyState::Available {
        empty_state(
            html,
            &format!(
                "{}: {:?}",
                text(
                    r.locale,
                    "Candidate Inspection dependency",
                    "Candidate Inspection 의존성"
                ),
                p.candidate_dependency
            ),
        );
    }
    for issue in p
        .issues
        .iter()
        .filter(|issue| issue.affected_scope == "candidate_inspection")
    {
        empty_state(
            html,
            &format!(
                "{} ({}: {})",
                issue.reason,
                text(r.locale, "omitted", "생략"),
                issue.omitted_count
            ),
        );
    }
    let learning = related_learning(p, w.work_item_id);
    if learning.is_empty() {
        empty_state(html, text(r.locale,
            "No permitted retained Learning association is visible for this Work. Withheld, expired or deleted content is not inferred.",
            "이 작업에 공개 가능한 보존 학습 연결이 없습니다. 비공개·만료·삭제 내용을 추론하지 않습니다."));
    }
    for candidate in learning {
        html.push_str(&format!("<article id=\"learning-{}\" data-candidate-id=\"{}\" data-work-id=\"{}\" data-statement-role=\"session-candidate\">",
            candidate.candidate_id, candidate.candidate_id, w.work_item_id));
        learning_inspection(html, r, candidate);
        html.push_str("</article>");
    }
    for q in p
        .resume
        .open_questions
        .iter()
        .filter(|q| w.open_question_ids.contains(&q.question_id))
    {
        html.push_str(&format!("<p>{}</p>", escape(&q.prompt)));
    }
    html.push_str("<p>");
    if snapshot {
        empty_state(html,text(r.locale,"Selected-Work code detail is omitted from this bounded snapshot. The repository view below is explicitly repository scope.","이 제한된 스냅샷에서는 선택 작업 코드 상세가 생략됩니다. 아래 저장소 보기는 명시적으로 저장소 범위입니다."));
    } else {
        link(
            html,
            r,
            ViewerView::Code {
                scope: CodeScope::Work(Some(w.work_item_id)),
                entity: None,
            },
            text(r.locale, "Explore this Work's code", "이 작업의 코드 탐색"),
            None,
        );
    }
    html.push_str("</p>");
    section_end(html);
}
fn decision_link(
    html: &mut String,
    r: &ViewerRequest,
    d: &volicord_projections::UnderstandingDecision,
    snapshot: bool,
) {
    html.push_str("<article class=\"decision-summary\"><h3>");
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
    html.push_str(&format!(
        "<article data-decision-scope=\"{}\">",
        decision_scope_key(d.decision.work_scope)
    ));
    section_start(
        html,
        &format!("decision-{}", d.decision.decision_id),
        text(r.locale, "Decision detail", "결정 상세"),
    );
    html.push_str(&format!(
        "<p class=\"state\">{} · {}</p>",
        escape(&decision_choice_attribution(&d.decision, r.locale)),
        escape(brief_decision_state_label(d.decision.state, r.locale))
    ));
    let answers = decision_answers(r, &d.decision);
    heading(html, 3, text(r.locale, "Choice", "선택"));
    empty_state(html, &decision_choice_attribution(&d.decision, r.locale));
    heading(
        html,
        3,
        text(
            r.locale,
            "Alternatives and expected trade-offs",
            "대안과 예상 절충",
        ),
    );
    html.push_str("<ul class=\"alternatives\">");
    for alternative in &d.decision.displayed_alternatives {
        list_item(
            html,
            &format!("{}: {}", alternative.label, alternative.consequence),
        );
    }
    html.push_str("</ul>");
    render_answers(html, r, &answers, false);
    if !snapshot {
        contextual_limits(html, r, p, false);
    }
    heading(
        html,
        3,
        text(r.locale, "Declared applicability", "선언된 적용 범위"),
    );
    html.push_str("<dl>");
    definition(
        html,
        text(r.locale, "Typed scope", "명시적 범위"),
        decision_scope_label(d.decision.work_scope, r.locale),
    );
    definition(
        html,
        text(r.locale, "Declared paths", "선언된 경로"),
        &d.declared_paths.join(", "),
    );
    html.push_str("</dl>");
    empty_state(
        html,
        text(
            r.locale,
            "Related code indicates scope overlap, not proof that this Decision was implemented.",
            "관련 코드는 범위 중첩이며 결정이 구현되었다는 증거가 아닙니다.",
        ),
    );
    for gap in d.known_link_gaps.iter().chain(&d.decision.known_limits) {
        empty_state(html, gap);
    }
    html.push_str("<details><summary>");
    html.push_str(text(
        r.locale,
        "Actual evidence and original rationale",
        "실제 근거 및 원래 이유",
    ));
    html.push_str("</summary><dl>");
    definition(
        html,
        "User Source",
        &join_ids(&d.decision.user_source_basis),
    );
    definition(
        html,
        "Recommendation Source",
        &join_ids(&d.decision.recommendation_source_basis),
    );
    definition(
        html,
        "Original user rationale",
        d.reading
            .user_rationale
            .original_text
            .as_deref()
            .unwrap_or(""),
    );
    definition(
        html,
        "Original recommendation rationale",
        d.reading
            .recommendation_rationale
            .original_text
            .as_deref()
            .unwrap_or(""),
    );
    definition(html, "Reading basis", &format!("{:?}", d.reading));
    html.push_str("</dl></details>");
    section_end(html);
    html.push_str("</article>");
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
    if let Some(work) = &p.selected_work {
        html.push_str("<div class=\"code-work-meaning\">");
        heading(html, 3, work_reading_display(&work.reading.goal, r.locale));
        let answers =
            volicord_projections::work_answers(work, &r.requested_language, r.locale.fixed());
        for answer in answers.prose.iter().filter(|a| {
            matches!(
                a.question.as_str(),
                "Purpose"
                    | "ReportedChange"
                    | "ExpectedEffect"
                    | "ExplanationAvailability"
                    | "Limits"
            )
        }) {
            heading(html, 4, answer_title(&answer.question, r.locale));
            html.push_str(&format!(
                "<p data-question=\"{}\">{}</p>",
                escape(&answer.question),
                escape(&answer.text)
            ));
        }
        html.push_str("</div>");
    }
    contextual_limits(html, r, p, snapshot);
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
        render_deterministic_explanation(html, r, explanation);
    }
    for interpretation in &u.generated_interpretations {
        html.push_str(&format!("<details data-statement-role=\"generated-interpretation\"><summary>{}</summary><p>{}</p><p>{}</p></details>",escape(text(r.locale,"Generated interpretation","생성 해석")),escape(&interpretation.text),escape(&interpretation.known_gaps.join("; "))));
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
    let entities = u
        .architecture
        .components
        .iter()
        .chain(&p.selected_entity_neighbors)
        .collect::<Vec<_>>();
    for relation in u
        .architecture
        .relationships
        .iter()
        .chain(&u.evidence.unresolved_relationships)
    {
        relation_detail(html, r, relation, &entities, snapshot);
    }
    if let Some(entity) = &p.selected_entity {
        heading(html, 3, &entity.display_name);
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
    }
    render_understanding_evidence(html, r, u);
    section_end(html);
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
            section_start(html, "works", text(r.locale, "Choose a Work", "작업 선택"));
            html.push_str("<div class=\"work-list\">");
            for w in &p.work_history {
                if p.selection.work_item_id == Some(w.work_item_id) {
                    html.push_str("<div class=\"current-work\"><p class=\"selection-label\">");
                    html.push_str(text(r.locale, "Latest recorded Work", "최근 기록된 작업"));
                    html.push_str("</p>");
                    work_summary(html, r, p, w, false);
                    html.push_str("</div>");
                } else {
                    work_summary(html, r, p, w, false);
                }
            }
            html.push_str("</div>");
            pagination(html, r, p.work_count, r.view.detail().work_page, true);
            section_end(html);
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
    section_start(html, "works", text(r.locale, "Work", "작업"));
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
    section_end(html);
    section_start(
        html,
        "decision-reading",
        text(r.locale, "Decisions", "결정"),
    );
    for d in &p.decision_catalog {
        decision_detail(html, r, d, p, true);
    }
    section_end(html);
    let repository_projection = p.for_repository_reading();
    let repository = build_project_understanding(
        &repository_projection,
        UnderstandingBound {
            max_items_per_section: 32,
        },
    );
    code(html, r, &repository_projection, &repository, true);
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
