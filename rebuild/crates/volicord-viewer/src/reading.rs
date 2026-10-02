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
                scope: CodeScope::Repository,
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
            text(request.locale, "Tools", "도구"),
            "health",
        ),
    ] {
        html.push_str("<li>");
        link(html, request, view, label, snapshot.then_some(id));
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
                text(request.locale, "Health and privacy", "상태 및 개인정보"),
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
pub(super) fn warnings(
    html: &mut String,
    request: &ViewerRequest,
    projection: &ProjectProjection,
    health: &volicord_operations::HealthReport,
) {
    section_start(
        html,
        "limitations",
        text(request.locale, "Material limitations", "중요한 한계"),
    );
    if health.state != HealthState::Healthy {
        html.push_str(&format!(
            "<p class=\"state\" data-state=\"{}\">{}: {}</p>",
            health_state_key(health.state),
            escape(text(request.locale, "Runtime health", "런타임 상태")),
            escape(health_state_label(health.state, request.locale))
        ));
    }
    if projection.sections.code == volicord_projections::ReadSectionState::NotRequested {
        empty_state(html, text(request.locale, "Code bodies not requested. Stored coverage and freshness remain visible; open Code Understanding for relationships.", "코드 본문은 요청하지 않음. 저장된 coverage와 freshness는 표시되며 관계는 코드 이해에서 확인하세요."));
    }
    html.push_str("<ul class=\"gap-list\">");
    for issue in &health.issues {
        list_item(html, &format!("{}: {}", issue.scope, issue.detail));
    }
    for issue in projection
        .issues
        .iter()
        .filter(|i| i.kind != ProjectionIssueKind::Bound)
        .take(32)
    {
        list_item(html, &format!("{}: {}", issue.affected_scope, issue.reason));
    }
    for item in &projection.resume.risks_assumptions_and_limits {
        list_item(html, &item.statement);
    }
    for limit in &projection.resume.known_limits {
        list_item(html, limit);
    }
    html.push_str("</ul>");
    if request.requested_language != locale_key(request.locale) {
        empty_state(html,text(request.locale,"Requested-language realization is unavailable. Original quotations preserve their source language; fixed labels use the selected locale.","요청 언어 실현을 사용할 수 없습니다. 원문 인용은 원래 언어를 유지하며 고정 설명은 선택한 UI 언어를 사용합니다."));
    }
    html.push_str("<details><summary>");
    html.push_str(text(request.locale, "Exact omissions", "정확한 생략"));
    html.push_str("</summary><ul>");
    for issue in projection.issues.iter().filter(|i| i.omitted_count > 0) {
        list_item(
            html,
            &format!(
                "{}: {} ({})",
                issue.affected_scope, issue.omitted_count, issue.reason
            ),
        );
    }
    html.push_str("</ul></details>");
    section_end(html);
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
            &u.current_work,
        ),
        (
            text(request.locale, "Recent outcomes", "최근 결과"),
            &u.completed_work,
        ),
        (
            text(request.locale, "Remaining Work", "남은 작업"),
            &u.remaining_work,
        ),
    ] {
        heading(html, 3, label);
        if works.is_empty() {
            empty_state(
                html,
                text(
                    request.locale,
                    "No recorded Work in this section.",
                    "이 항목에 기록된 작업이 없습니다.",
                ),
            );
        }
        for work in works.iter().take(8) {
            work_summary(html, request, work, snapshot);
        }
        if works.len() > 8 {
            list_item(
                html,
                &format!(
                    "{} {}",
                    works.len() - 8,
                    text(
                        request.locale,
                        "additional Works; open Work navigation",
                        "추가 작업; 작업 탐색을 여세요"
                    )
                ),
            );
        }
    }
    heading(
        html,
        3,
        text(request.locale, "Recorded next steps", "기록된 다음 단계"),
    );
    for work in u
        .work_history
        .iter()
        .filter(|w| w.next_step.is_some())
        .take(8)
    {
        html.push_str("<p class=\"next-action\">");
        html.push_str(&escape(work_reading_display(
            &work.reading.next_step,
            request.locale,
        )));
        html.push_str("</p>");
    }
    if p.work_history.iter().all(|w| w.next_step.is_none()) {
        empty_state(
            html,
            text(
                request.locale,
                "Next step unavailable: no Checkpoint records one.",
                "다음 단계 없음: 기록한 체크포인트가 없습니다.",
            ),
        );
    }
    for unresolved in &u.unresolved_work_grouping {
        empty_state(html, &unresolved.reason);
    }
    section_end(html);
}
fn work_summary(html: &mut String, r: &ViewerRequest, w: &UnderstandingWork, snapshot: bool) {
    html.push_str(&format!("<article class=\"understanding-card work-item\" data-work-id=\"{}\" data-work-state=\"{}\">",w.work_item_id,understanding_work_state_key(w.state)));
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
    html.push_str(&format!(
        "<p>{}</p>",
        escape(understanding_work_state_label(w.state, r.locale))
    ));
    if let Some(change) = w.reading.changes.last() {
        html.push_str(&format!(
            "<p>{}: {}</p>",
            escape(text(
                r.locale,
                "Recorded result / change",
                "기록된 결과 / 변경"
            )),
            escape(work_reading_display(change, r.locale))
        ));
    }
    if let Some(state) = w.reading.states.last() {
        states(html, r, state);
    }
    html.push_str("</article>");
}
fn states(html: &mut String, r: &ViewerRequest, s: &volicord_projections::WorkStateObservation) {
    html.push_str("<dl class=\"fact-states\" data-statement-role=\"deterministic-derived\">");
    definition(
        html,
        text(r.locale, "Work", "작업"),
        &format!("{:?}", s.work_state),
    );
    definition(
        html,
        text(r.locale, "Automated verification", "자동 검증"),
        &format!(
            "{:?}",
            s.verification.iter().map(|v| v.state).collect::<Vec<_>>()
        ),
    );
    definition(
        html,
        text(r.locale, "User review", "사용자 검토"),
        &format!("{:?}", s.user_review.state),
    );
    definition(
        html,
        text(r.locale, "User acceptance", "사용자 수락"),
        &format!("{:?}", s.user_acceptance.state),
    );
    html.push_str("</dl>");
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
    empty_state(html,text(r.locale,"Semantic summary unavailable: Goal and changes are original quotations or labeled excerpts.","의미 요약 없음: 목표와 변경은 원문 인용 또는 표시된 발췌입니다."));
    heading(
        html,
        3,
        text(r.locale, "Result and next step", "결과 및 다음 단계"),
    );
    if w.reading.changes.is_empty() {
        empty_state(
            html,
            text(
                r.locale,
                "Result unavailable: Goal only, no Checkpoint.",
                "결과 없음: 체크포인트 없이 목표만 기록됨.",
            ),
        );
    }
    html.push_str(&format!(
        "<p>{}</p>",
        escape(work_reading_display(&w.reading.next_step, r.locale))
    ));
    if let Some(state) = w.reading.states.last() {
        states(html, r, state);
    }
    for state in &w.reading.states {
        if state
            .verification
            .iter()
            .any(|v| v.state == VerificationState::Failed)
            || state.user_acceptance.state == UserAcceptanceState::Rejected
        {
            empty_state(
                html,
                &format!(
                    "{} — {:?}; {:?}",
                    text(
                        r.locale,
                        "Historical failed / rejected observation",
                        "과거 실패 / 거절 관찰"
                    ),
                    state
                        .verification
                        .iter()
                        .map(|v| v.state)
                        .collect::<Vec<_>>(),
                    state.user_acceptance.state
                ),
            );
        }
    }
    html.push_str("<details><summary>");
    html.push_str(text(
        r.locale,
        "Verification and original state observations",
        "검증 및 원래 상태 관찰",
    ));
    html.push_str("</summary>");
    for state in &w.reading.states {
        states(html, r, state);
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
    html.push_str("<p>");
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
    html.push_str("</p>");
}
fn decision_detail(
    html: &mut String,
    r: &ViewerRequest,
    d: &volicord_projections::UnderstandingDecision,
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
    html.push_str("<dl>");
    definition(
        html,
        text(r.locale, "User rationale", "사용자 근거"),
        work_reading_display(&d.reading.user_rationale, r.locale),
    );
    definition(
        html,
        text(r.locale, "Agent recommendation", "에이전트 권고"),
        &recommendation_attribution(&d.decision, r.locale),
    );
    definition(
        html,
        text(r.locale, "Recommendation rationale", "권고 근거"),
        work_reading_display(&d.reading.recommendation_rationale, r.locale),
    );
    definition(
        html,
        text(r.locale, "Alternative consequences", "대안별 예상 결과"),
        &alternative_consequences(&d.decision, r.locale),
    );
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
    for gap in d
        .known_link_gaps
        .iter()
        .chain(&d.decision.known_limits)
        .chain(&d.decision.review_basis)
    {
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
    definition(html, "Reading basis", &format!("{:?}", d.reading));
    html.push_str("</dl></details>");
    section_end(html);
    html.push_str("</article>");
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
    render_grounded_diagram(
        html,
        r,
        u,
        "flow-topology",
        text(r.locale, "Grounded flow evidence", "근거 있는 흐름 증거"),
        is_flow_relation,
        false,
    );
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
            empty_state(
                html,
                &format!(
                    "{} {}",
                    p.omitted_selected_relation_count,
                    text(r.locale, "relationships omitted", "관계 생략")
                ),
            );
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
            for w in &p.work_history {
                work_summary(html, r, w, false);
            }
            pagination(html, r, p.work_count, r.view.detail().work_page, true);
            section_end(html);
        }
        ViewerView::Decisions { decision: Some(_) } => {
            if let Some(d) = &p.selected_decision {
                decision_detail(html, r, d);
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
    for w in &p.work_history {
        work_detail(html, r, p, w, true);
    }
    section_end(html);
    section_start(
        html,
        "decision-reading",
        text(r.locale, "Decisions", "결정"),
    );
    for d in &p.decision_catalog {
        decision_detail(html, r, d);
    }
    section_end(html);
    // Reuse immutable materialized data; never load a separate per-Work basis.
    let mut repository_projection = p.clone();
    repository_projection.selection = volicord_projections::WorkSelection {
        selector: volicord_projections::WorkSelector::Repository,
        work_item_id: None,
        basis: volicord_projections::WorkSelectionBasis::Repository,
    };
    repository_projection.selected_work = None;
    repository_projection.selected_work_decisions.clear();
    repository_projection.current_work_code.clear();
    repository_projection.current_work_topology.entities = p.repository_map.entities.clone();
    repository_projection.current_work_topology.relations = p.repository_map.relations.clone();
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
    html.push_str("</p></details></details>");
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
