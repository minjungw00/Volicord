//! Server-rendered catalogue and Overview in the common project shell.
use super::*;
use crate::ViewerView;
use volicord_projections::{
    CatalogueArticle, WorkCatalogueEntry, WorkCatalogueOrder, WorkCataloguePage,
    WorkCatalogueQuery, WORK_CATALOGUE_PAGE_SIZE,
};

fn href(r: &ViewerRequest, query: WorkCatalogueQuery) -> String {
    reading::url(r, &ViewerView::WorkQuery { query })
}
fn name<'a>(e: &'a WorkCatalogueEntry, r: &ViewerRequest) -> &'a str {
    e.name
        .as_deref()
        .unwrap_or(text(r.locale, "Recorded Work", "작업 기록"))
}
fn summary<'a>(e: &'a WorkCatalogueEntry, r: &ViewerRequest) -> &'a str {
    e.summary.as_deref().unwrap_or(text(r.locale,"Work summary is unavailable. Read the original instructions and available records in Work detail.","작업 요약을 사용할 수 없습니다. 작업 상세에서 원래 지시와 사용 가능한 기록을 읽을 수 있습니다."))
}
fn date(html: &mut String, r: &ViewerRequest, time: Option<i64>) {
    if let Some((time, (instant, calendar))) = time.and_then(|time| {
        observation_time(TimestampMicros::from_unix_micros(time)).map(|value| (time, value))
    }) {
        html.push_str(&format!(
            "<time datetime=\"{instant}\" data-recorded-at=\"{time}\" title=\"Unix microseconds: {time}\">{calendar}</time>"
        ));
    } else {
        html.push_str(text(r.locale, "Date unavailable", "날짜 확인 불가"));
    }
}
fn row(
    html: &mut String,
    r: &ViewerRequest,
    e: &WorkCatalogueEntry,
    origin: Option<&WorkCatalogueQuery>,
) {
    let target = reading::url(
        r,
        &origin.map_or(ViewerView::Work { work: Some(e.id) }, |query| {
            ViewerView::WorkContext {
                work: e.id,
                query: query.clone(),
            }
        }),
    );
    let article = match e.article {
        CatalogueArticle::Missing => text(r.locale, "Records only", "진행 기록만"),
        CatalogueArticle::Recorded => text(
            r.locale,
            "Article recorded · checked on open",
            "설명 기록 있음 · 열 때 확인",
        ),
        CatalogueArticle::Unavailable => text(r.locale, "Article unavailable", "설명 사용 불가"),
    };
    html.push_str(&format!("<a class=\"work-row\" id=\"row-{}\" data-work-link=\"{}\" data-work-id=\"{}\" data-work-state=\"{}\" href=\"{}\"><div class=\"work-main\"><div class=\"work-row-kicker\"><code>{}</code></div><h3 lang=\"{}\" data-title-basis=\"{}\">{}</h3><p lang=\"{}\">{}</p></div><div class=\"work-row-state\"><span class=\"work-badge\" data-state=\"{}\">{}</span><span class=\"article-state\">{article}</span></div><div class=\"work-row-date\">",e.id,e.id,e.id,work_state_key(e.state),escape(&target),e.id,if e.name.is_some(){escape(&content_language_tag(&r.requested_language))}else{locale_key(r.locale).into()},if e.name.is_some(){"generated-interpretation"}else{"neutral-label"},escape(name(e,r)),if e.summary.is_some(){escape(&content_language_tag(&r.requested_language))}else{locale_key(r.locale).into()},escape(summary(e,r)),work_state_key(e.state),understanding_work_state_label(e.state,r.locale)));
    date(html, r, e.updated_at);
    html.push_str(&format!(
        "<small>{} {}</small></div><span class=\"row-arrow\" aria-hidden=\"true\">›</span></a>",
        e.checkpoint_count,
        text(r.locale, "Checkpoint records", "개 진행 기록")
    ));
}
fn states() -> [UnderstandingWorkState; 6] {
    [
        UnderstandingWorkState::InProgress,
        UnderstandingWorkState::Paused,
        UnderstandingWorkState::Completed,
        UnderstandingWorkState::Abandoned,
        UnderstandingWorkState::Open,
        UnderstandingWorkState::Superseded,
    ]
}
pub(super) fn index(
    html: &mut String,
    r: &ViewerRequest,
    p: &ProjectProjection,
    c: &WorkCataloguePage,
) {
    html.push_str(&format!("<section class=\"snapshot-page catalogue-page\" id=\"works\" data-catalogue-route=\"{}\" data-catalogue-basis=\"{}\"><header class=\"page-header\"><p class=\"eyebrow\">{}</p><div class=\"heading-count\"><h1 class=\"index-title\">{}</h1><span>{}</span></div><p class=\"overview-lead\">{}</p></header>",escape(&href(r,c.query.clone())),c.basis,escape(&p.overview.project_name),text(r.locale,"Work","작업"),p.work_count,text(r.locale,"Find and read recorded goals, progress and investigations.","진행 중인 일부터 지난 조사까지, 목표와 결과를 함께 찾아 읽습니다.")));
    html.push_str("<form class=\"catalog-controls\" method=\"get\" action=\"/\" role=\"search\">");
    for (key, value) in [
        ("project", r.project_id.to_string()),
        ("view", "work".into()),
        ("locale", locale_key(r.locale).into()),
        ("language", r.requested_language.clone()),
    ] {
        html.push_str(&format!(
            "<input type=\"hidden\" name=\"{key}\" value=\"{}\">",
            escape(&value)
        ));
    }
    if let Some(state) = c.query.state {
        html.push_str(&format!(
            "<input type=\"hidden\" name=\"state\" value=\"{}\">",
            work_state_key(state)
        ));
    }
    html.push_str(&format!("<div class=\"search-field\"><label class=\"sr-only\" for=\"work-search\">{}</label><input id=\"work-search\" type=\"search\" name=\"q\" maxlength=\"128\" value=\"{}\" placeholder=\"{}\" aria-describedby=\"search-scope\"><button type=\"submit\">{}</button></div><label class=\"sort-field\"><span class=\"sr-only\">{}</span><select name=\"sort\">",text(r.locale,"Search Work name or summary","작업 이름이나 요약으로 찾기"),escape(&c.query.term),text(r.locale,"Search name or summary","작업 이름이나 요약으로 찾기"),text(r.locale,"Search","검색"),text(r.locale,"Order","정렬")));
    for (order, key, label) in [
        (
            WorkCatalogueOrder::Recent,
            "recent",
            text(r.locale, "Recent update", "최근 변경순"),
        ),
        (
            WorkCatalogueOrder::Oldest,
            "oldest",
            text(r.locale, "Oldest update", "오래된 순"),
        ),
        (
            WorkCatalogueOrder::Name,
            "name",
            text(r.locale, "Name", "이름순"),
        ),
    ] {
        html.push_str(&format!(
            "<option value=\"{key}\"{}>{label}</option>",
            if c.query.order == order {
                " selected"
            } else {
                ""
            }
        ));
    }
    html.push_str(&format!("</select></label></form><p class=\"search-scope\" id=\"search-scope\">{}</p>",text(r.locale,"Searches permitted names and summaries in the requested content language. State counts cover the entire Project.","요청한 본문 언어의 허용된 이름·요약을 검색합니다. 상태별 수는 프로젝트 전체 기준입니다.")));
    html.push_str(&format!(
        "<nav class=\"status-filters\" aria-label=\"{}\">",
        text(r.locale, "Work state filter", "작업 상태 필터")
    ));
    for state in std::iter::once(None).chain(states().into_iter().map(Some)) {
        let mut query = c.query.clone();
        query.state = state;
        query.page = 0;
        query.expected_basis = None;
        let label = state.map_or(text(r.locale, "All", "전체"), |s| {
            understanding_work_state_label(s, r.locale)
        });
        let count = state.map_or(p.work_count, |s| {
            p.work_state_counts.get(&s).copied().unwrap_or(0)
        });
        html.push_str(&format!(
            "<a class=\"filter-tab\" href=\"{}\"{}>{label}<span>{count}</span></a>",
            escape(&href(r, query)),
            if state == c.query.state {
                " aria-current=\"true\""
            } else {
                ""
            }
        ));
    }
    html.push_str("</nav>");
    if c.refreshed {
        html.push_str(&format!(
            "<p class=\"catalogue-refresh\" role=\"status\">{}</p>",
            text(
                r.locale,
                "Records or query changed. Showing the first page of the current results.",
                "기록 또는 검색 조건이 바뀌어 현재 결과의 첫 페이지를 표시합니다."
            )
        ));
    }
    let start = if c.entries.is_empty() {
        0
    } else {
        c.query.page * WORK_CATALOGUE_PAGE_SIZE + 1
    };
    html.push_str(&format!("<div class=\"result-summary\" role=\"status\" data-result-count=\"{}\"><span>{}: {} · {}–{} {}</span><a href=\"{}\">{}</a></div>",c.matched_count,text(r.locale,"Results","검색 결과"),c.matched_count,start,start.saturating_sub(1)+c.entries.len(),text(r.locale,"shown","표시"),escape(&href(r,WorkCatalogueQuery::default())),text(r.locale,"Reset","조건 초기화")));
    html.push_str(&format!("<div class=\"work-list-header\" aria-hidden=\"true\"><span>{}</span><span>{}</span><span>{}</span></div><div class=\"work-list\">",text(r.locale,"Work · summary","작업 · 요약"),text(r.locale,"State","상태"),text(r.locale,"Latest record","최근 기록")));
    for entry in &c.entries {
        row(html, r, entry, Some(&c.query));
    }
    if c.entries.is_empty() {
        html.push_str(&format!(
            "<div class=\"catalogue-empty\"><h2>{}</h2><p>{}</p></div>",
            if p.work_count == 0 {
                text(
                    r.locale,
                    "No Work has been recorded.",
                    "기록된 작업이 없습니다.",
                )
            } else {
                text(r.locale, "No matching Work", "조건에 맞는 작업이 없습니다.")
            },
            text(
                r.locale,
                "Choose another term or state filter.",
                "다른 단어로 찾거나 상태 필터를 바꿔보세요."
            )
        ));
    }
    html.push_str("</div>");
    let pages = c.matched_count.div_ceil(WORK_CATALOGUE_PAGE_SIZE);
    if pages > 1 {
        html.push_str(&format!(
            "<nav class=\"pagination\" aria-label=\"{}\">",
            text(r.locale, "Work list pages", "작업 목록 페이지")
        ));
        let mut targets = std::collections::BTreeSet::from([0, pages - 1]);
        targets.extend(c.query.page.saturating_sub(2)..=(c.query.page + 2).min(pages - 1));
        let mut previous = None;
        for page in targets {
            if previous.is_some_and(|n| n + 1 < page) {
                html.push_str("<span aria-hidden=\"true\">…</span>");
            }
            let mut query = c.query.clone();
            query.page = page;
            html.push_str(&format!(
                "<a href=\"{}\"{} aria-label=\"{} {}\">{}</a>",
                escape(&href(r, query)),
                if page == c.query.page {
                    " aria-current=\"page\""
                } else {
                    ""
                },
                text(r.locale, "Page", "페이지"),
                page + 1,
                page + 1
            ));
            previous = Some(page);
        }
        html.push_str("</nav>");
    }
    html.push_str("</section>");
}

pub(super) fn overview(
    html: &mut String,
    r: &ViewerRequest,
    p: &ProjectProjection,
    u: &ProjectUnderstanding,
    c: &WorkCataloguePage,
) {
    html.push_str(&format!("<section id=\"overview\" class=\"snapshot-page overview-page\"><header class=\"page-header\"><p class=\"eyebrow\">PROJECT OVERVIEW</p><div class=\"project-title\"><span class=\"project-emblem\" aria-hidden=\"true\">{}</span><div><h1>{}</h1><p class=\"project-meta\">{}</p></div></div>",escape(&p.overview.project_name.chars().next().unwrap_or('V').to_string()),escape(&p.overview.project_name),text(r.locale,"Local Project · retained records","로컬 프로젝트 · 보존된 기록")));
    reading::project_purpose(html, r, u);
    html.push_str("</header><div class=\"overview-grid\"><section class=\"continue-card\">");
    if let Some(work) = c.current.as_ref().or(c.entries.first()) {
        html.push_str(&format!("<p class=\"card-topline\">{}</p><h2>{}</h2><p lang=\"{}\">{}</p><div class=\"card-bottom\"><small>{} · ",if c.current.is_some(){text(r.locale,"Recent work in progress","최근 진행 중인 작업")}else{text(r.locale,"Most recently recorded Work","최근 기록된 작업")},escape(name(work,r)),escape(&content_language_tag(&r.requested_language)),escape(summary(work,r)),understanding_work_state_label(work.state,r.locale)));
        date(html, r, work.updated_at);
        html.push_str(&format!(
            "</small><a class=\"button primary\" data-work-link=\"{}\" href=\"{}\">{} →</a></div>",
            work.id,
            escape(&reading::url(
                r,
                &ViewerView::Work {
                    work: Some(work.id)
                }
            )),
            text(r.locale, "Read Work", "작업 읽기")
        ));
    } else {
        html.push_str(&format!(
            "<h2>{}</h2><p>{}</p>",
            text(
                r.locale,
                "No Work has been recorded.",
                "기록된 작업이 없습니다."
            ),
            text(
                r.locale,
                "Recorded Work will appear here.",
                "작업을 기록하면 이곳에서 읽을 수 있습니다."
            )
        ));
    }
    html.push_str(&format!("</section><section class=\"repository-card\"><p class=\"card-topline\">{}</p><h2>{}</h2><p>{}</p><a class=\"text-action repository-reading-link\" href=\"{}\">{} →</a></section></div>",text(r.locale,"Repository explanation","저장소 설명"),text(r.locale,"Start with the structure","전체 구조부터 살펴보기"),text(r.locale,"Open the available repository article and inspect the code basis and limits stated there.","저장소 글을 열어 설명한 구조와 동작, 코드 기준과 한계를 확인하세요."),escape(&reading::url(r,&ViewerView::RepositoryExplanation)),text(r.locale,"Read repository explanation","저장소 설명 읽기")));
    html.push_str(&format!(
        "<div class=\"work-stats\"><a href=\"{}\">{} <b>{}</b></a>",
        escape(&href(r, WorkCatalogueQuery::default())),
        text(r.locale, "Project Work", "프로젝트 작업"),
        p.work_count
    ));
    for state in states() {
        let count = p.work_state_counts.get(&state).copied().unwrap_or(0);
        if count > 0 {
            html.push_str(&format!(
                "<a href=\"{}\">{} <b>{count}</b></a>",
                escape(&href(
                    r,
                    WorkCatalogueQuery {
                        state: Some(state),
                        ..Default::default()
                    }
                )),
                understanding_work_state_label(state, r.locale)
            ));
        }
    }
    html.push_str(&format!("</div><div class=\"list-section-heading\"><div><h2>{}</h2><p>{}</p></div><a href=\"{}\">{} →</a></div><div class=\"work-list recent\">",text(r.locale,"Recent Work","최근 작업"),text(r.locale,"Latest recorded update first","마지막으로 기록한 순서대로 읽습니다."),escape(&href(r,WorkCatalogueQuery::default())),text(r.locale,"All Work","전체 작업 보기")));
    for entry in c.entries.iter().take(5) {
        row(html, r, entry, None);
    }
    html.push_str("</div></section>");
}
