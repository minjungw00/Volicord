//! Shared project navigation. Project identity is independent of authored prose.
use super::{presentation::*, ViewerRequest};
use crate::ViewerView;
use volicord_context::Project;
use volicord_projections::{ProjectProjection, UnderstandingWorkState};

pub(crate) fn work_state_key(state: UnderstandingWorkState) -> &'static str {
    match state {
        UnderstandingWorkState::Open => "open",
        UnderstandingWorkState::InProgress => "in_progress",
        UnderstandingWorkState::Paused => "paused",
        UnderstandingWorkState::Completed => "completed",
        UnderstandingWorkState::Abandoned => "abandoned",
        UnderstandingWorkState::Superseded => "superseded",
    }
}
fn href(r: &ViewerRequest, view: &ViewerView, snapshot: bool) -> String {
    if snapshot {
        return match view {
            ViewerView::Overview => "#overview",
            ViewerView::RepositoryExplanation => "#repository-explanation",
            _ => "#works",
        }
        .into();
    }
    let mut pairs = view.fields();
    pairs.extend([
        ("project", r.project_id.to_string()),
        ("locale", locale_key(r.locale).into()),
        ("language", r.requested_language.clone()),
    ]);
    format!(
        "/?{}",
        pairs
            .into_iter()
            .map(|(k, v)| format!("{k}={}", percent_encode(&v)))
            .collect::<Vec<_>>()
            .join("&")
    )
}
fn page_label(r: &ViewerRequest) -> &'static str {
    match r.view {
        ViewerView::Overview => text(r.locale, "Overview", "개요"),
        ViewerView::RepositoryExplanation => {
            text(r.locale, "Repository explanation", "저장소 설명")
        }
        _ => text(r.locale, "Work", "작업"),
    }
}
fn icon(kind: usize) -> &'static str {
    match kind {
        0 => "<path d=\"M2 3l6-1 6 1v11l-6-1-6 1zM8 2v11\"/>",
        1 => "<path d=\"M5 4L1 8l4 4m6-8l4 4-4 4M9 1L7 15\"/>",
        _ => "<path d=\"M1 4h5l2 2h7v8H1z\"/>",
    }
}
pub(super) fn start(
    html: &mut String,
    r: &ViewerRequest,
    p: &ProjectProjection,
    choices: &[Project],
    snapshot: bool,
) {
    let name = escape(&p.overview.project_name);
    let location = if snapshot {
        [
            ("overview", text(r.locale, "Overview", "개요")),
            (
                "repository",
                text(r.locale, "Repository explanation", "저장소 설명"),
            ),
            ("work", text(r.locale, "Work", "작업")),
            ("support", text(r.locale, "Export basis", "내보내기 기준")),
            ("all", text(r.locale, "Exported pages", "내보낸 문서들")),
        ]
        .into_iter()
        .map(|(kind, label)| format!("<span data-page-context=\"{kind}\">{label}</span>"))
        .collect::<String>()
    } else {
        page_label(r).into()
    };
    let close_label = text(r.locale, "Close project menu", "프로젝트 메뉴 닫기");
    let rail_close = if snapshot {
        String::new()
    } else {
        format!("<button class=\"rail-close\" type=\"button\" hidden aria-label=\"{close_label}\">×</button>")
    };
    html.push_str(&format!("<a class=\"skip skip-link\" href=\"#viewer-content\">{}</a><details class=\"project-shell\"><summary class=\"mobile-header\"><span class=\"menu-label\">{}</span><span class=\"menu-glyph\" aria-hidden=\"true\">☰</span><span class=\"mobile-project\">{name}</span><span class=\"mobile-location\">{location}</span></summary><aside class=\"project-rail\" aria-label=\"{}\"><div class=\"rail-top\"><div class=\"brand-line\"><a class=\"brand\" href=\"{}\"><span class=\"v-mark\" aria-hidden=\"true\">V</span>volicord</a>{rail_close}</div><p class=\"rail-heading\">{}</p>",text(r.locale,"Skip to content","본문으로 건너뛰기"),text(r.locale,"Open project menu","프로젝트 메뉴 열기"),text(r.locale,"Project navigation","프로젝트 내 이동"),escape(&href(r,&ViewerView::Overview,snapshot)),text(r.locale,"Project","프로젝트")));
    html.push_str(&format!("<details class=\"project-picker\"><summary><span class=\"project-mark\" aria-hidden=\"true\">{}</span><span>{name}<small>{}</small></span><span aria-hidden=\"true\">⌄</span></summary>",escape(&p.overview.project_name.chars().next().unwrap_or('V').to_string()),text(r.locale,"Retained local context","보존된 로컬 맥락")));
    if snapshot {
        html.push_str(&format!(
            "<p class=\"project-selection-note\">{}</p>",
            text(
                r.locale,
                "This export contains this Project only.",
                "이 내보내기에는 이 프로젝트만 포함됩니다."
            )
        ));
    } else {
        html.push_str(&format!(
            "<nav aria-label=\"{}\">",
            text(r.locale, "Choose Project", "프로젝트 선택")
        ));
        for project in choices {
            let mut selected = r.clone();
            selected.project_id = project.id;
            html.push_str(&format!(
                "<a href=\"{}\"{}>{}<small>{}</small></a>",
                escape(&href(&selected, &ViewerView::Overview, false)),
                if project.id == r.project_id {
                    " aria-current=\"true\""
                } else {
                    ""
                },
                escape(&project.display_name),
                project.id
            ));
        }
        html.push_str("</nav>");
        // A bounded list must not imply that other local identities do not exist.
        if choices.len() == 32 {
            html.push_str(&format!("<form method=\"get\" action=\"/\"><label>{}<input name=\"project\" pattern=\"[0-9a-fA-F]{{32}}\" required></label><input type=\"hidden\" name=\"view\" value=\"overview\"><input type=\"hidden\" name=\"locale\" value=\"{}\"><input type=\"hidden\" name=\"language\" value=\"{}\"><button>{}</button></form>",text(r.locale,"First 32 local Projects · open another by ID","로컬 프로젝트 첫 32개 · 다른 ID로 열기"),locale_key(r.locale),escape(&r.requested_language),text(r.locale,"Open Project","프로젝트 열기")));
        }
    }
    html.push_str("</details></div><div class=\"rail-navigation\"><nav class=\"project-navigation\" aria-label=\"Viewer\">");
    for (i, view, label) in [
        (0, ViewerView::Overview, text(r.locale, "Overview", "개요")),
        (
            1,
            ViewerView::RepositoryExplanation,
            text(r.locale, "Repository explanation", "저장소 설명"),
        ),
        (
            2,
            ViewerView::Work { work: None },
            text(r.locale, "Work", "작업"),
        ),
    ] {
        // CSS exposes the accessible cue on the owning native destination.
        // aria-current cannot be frozen to the export request.
        let current = if snapshot {
            format!(
                "<span class=\"native-current\"> · {}</span>",
                text(r.locale, "Current page", "현재 페이지")
            )
        } else {
            String::new()
        };
        html.push_str(&format!("<a class=\"nav-link\" data-destination=\"{}\" href=\"{}\"{}><svg viewBox=\"0 0 16 16\" width=\"16\" height=\"16\" fill=\"none\" stroke=\"currentColor\" aria-hidden=\"true\">{}</svg>{label}{}{current} </a>",view.key(),escape(&href(r,&view,snapshot)),if !snapshot && view.key()==r.view.key(){" aria-current=\"page\""}else{""},icon(i),if i==2{format!("<span class=\"nav-count\">{}</span>",p.work_count)}else{String::new()}));
    }
    html.push_str(&format!(
        "</nav><div class=\"rail-states\"><p class=\"rail-heading\">{}</p>",
        text(r.locale, "Work by state", "상태별 작업")
    ));
    for state in [
        UnderstandingWorkState::InProgress,
        UnderstandingWorkState::Paused,
        UnderstandingWorkState::Completed,
        UnderstandingWorkState::Abandoned,
        UnderstandingWorkState::Open,
        UnderstandingWorkState::Superseded,
    ] {
        let count = p.work_state_counts.get(&state).copied().unwrap_or(0);
        if count == 0
            && matches!(
                state,
                UnderstandingWorkState::Open | UnderstandingWorkState::Superseded
            )
        {
            continue;
        }
        let label = super::understanding_work_state_label(state, r.locale);
        html.push_str(&format!("<a class=\"status-side-link\" data-state=\"{}\" href=\"{}\"{}><span class=\"state-dot\" aria-hidden=\"true\"></span>{}<span class=\"nav-count\">{count}</span></a>",work_state_key(state),escape(&href(r,&ViewerView::WorkFilter {state,page:0},snapshot)),if r.view.detail().work_state==Some(state){" aria-current=\"true\""}else{""},escape(label)));
    }
    let backdrop = if snapshot {
        String::new()
    } else {
        format!("<button class=\"nav-backdrop\" hidden type=\"button\" tabindex=\"-1\" aria-label=\"{close_label}\"></button>")
    };
    html.push_str(&format!("</div></div><footer class=\"rail-bottom\"><p>{}</p><p>{}</p><small>{}: {}</small></footer></aside></details>{backdrop}<main class=\"shell viewer-shell\" id=\"viewer-content\" tabindex=\"-1\">",text(r.locale,"Read-only","읽기 전용"),if snapshot{text(r.locale,"Export-time local context","내보낸 시점의 로컬 맥락")}else{text(r.locale,"Retained local records","보존된 로컬 기록")},text(r.locale,"Canonical context revision","정식 맥락 리비전"),p.overview.canonical_revision));
}
