//! Presentation for deliberate record inspection; read authority stays in Operations.
use super::*;
use crate::inspection::InspectionSelection;
use volicord_operations::{
    InspectionClass, InspectionPage, InspectionRow, INSPECTION_ROLES, INSPECTION_STATES,
};

fn class_name(class: InspectionClass, locale: ViewerLocale) -> &'static str {
    use InspectionClass::*;
    match class {
        Work => text(locale, "Work / Goal", "작업 / 목표"),
        Context => text(locale, "Context & purpose", "맥락 및 목적"),
        Checkpoint => text(locale, "Checkpoints", "체크포인트"),
        Decision => text(locale, "Decisions", "결정"),
        Question => text(locale, "Questions", "질문"),
        Source => text(locale, "Sources", "Source"),
        Candidate => text(locale, "Session Candidates", "세션 Candidate"),
        Derived => text(locale, "Derived metadata", "파생 데이터 메타정보"),
    }
}
fn base_fields(r: &ViewerRequest, s: &InspectionSelection) -> Vec<(&'static str, String)> {
    let q = &s.query;
    let mut fields = vec![
        ("class", q.class.key().into()),
        ("locale", locale_key(r.locale).into()),
        ("language", r.requested_language.clone()),
        ("limit", q.limit.to_string()),
    ];
    if !q.term.is_empty() {
        fields.push(("q", q.term.clone()));
    }
    if let Some(v) = &q.state {
        fields.push(("state", v.clone()));
    }
    if let Some(v) = &q.role {
        fields.push(("role", v.clone()));
    }
    fields
}
fn href(fields: &[(&str, String)]) -> String {
    format!(
        "/inspection?{}",
        fields
            .iter()
            .map(|(k, v)| format!("{k}={}", percent_encode(v)))
            .collect::<Vec<_>>()
            .join("&")
    )
}
fn anchor(html: &mut String, fields: &[(&str, String)], label: &str) {
    html.push_str(&format!(
        "<a class=\"button\" href=\"{}\">{}</a>",
        escape(&href(fields)),
        escape(label)
    ));
}
fn select(
    html: &mut String,
    name: &str,
    label: &str,
    value: &str,
    values: impl Iterator<Item = (String, String)>,
) {
    html.push_str(&format!(
        "<label>{}<select name=\"{}\">",
        escape(label),
        escape(name)
    ));
    for (key, title) in values {
        html.push_str(&format!(
            "<option value=\"{}\"{}>{}</option>",
            escape(&key),
            if value == key { " selected" } else { "" },
            escape(&title)
        ));
    }
    html.push_str("</select></label>");
}
fn records_for_actions(
    rows: &[InspectionRow],
) -> Vec<volicord_projections::CanonicalInspectionItem> {
    rows.iter()
        .filter_map(|r| {
            Some(volicord_projections::CanonicalInspectionItem {
                kind: match r.class {
                    InspectionClass::Work | InspectionClass::Context => {
                        CanonicalInspectionKind::ContextItem
                    }
                    InspectionClass::Checkpoint => CanonicalInspectionKind::Checkpoint,
                    InspectionClass::Decision => CanonicalInspectionKind::Decision,
                    InspectionClass::Question => CanonicalInspectionKind::Question,
                    InspectionClass::Source => CanonicalInspectionKind::Source,
                    _ => return None,
                },
                identity: r.identity.clone(),
                revision: r.revision?,
                lifecycle_state: r.state.clone(),
                statement_role: Some(r.role.clone()),
                summary: r.summary.clone(),
                source_basis: Vec::new(),
            })
        })
        .collect()
}

pub(crate) fn inspection_page(
    adapter: &ViewerAdapter,
    r: &ViewerRequest,
    s: &InspectionSelection,
    page: InspectionPage,
    detail: Option<serde_json::Value>,
    token: &str,
    projection: Option<&ProjectProjection>,
) -> ViewerPage {
    let mut html=format!("<!doctype html><html lang=\"{}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\"><title>{} · Volicord</title>",locale_key(r.locale),escape(text(r.locale,"Project records","프로젝트 기록")));
    // Retain the actual Product observation binding without loading graph/Candidates.
    if let Some(projection) = projection {
        if let Ok(context) =
            crate::observation::context(adapter.operations().layout().root(), r, projection)
        {
            html.push_str(&format!(
                "<meta name=\"volicord-observation\" content=\"{}\">",
                escape(&context.to_string())
            ));
        }
    }
    html.push_str(STYLE);
    html.push_str(&format!("</head><body data-view=\"tools\" data-viewer-mode=\"live\"><a class=\"skip skip-link\" href=\"#viewer-content\">{}</a><header class=\"masthead viewer-header\"><div class=\"shell\"><a class=\"brand\" href=\"/?view=overview&amp;locale={}&amp;language={}\">Volicord</a>",text(r.locale,"Skip to content","본문으로 건너뛰기"),locale_key(r.locale),escape(&percent_encode(&r.requested_language))));
    reading::navigation(&mut html, r, false);
    html.push_str("</div></header><main class=\"shell viewer-shell inspector\" id=\"viewer-content\" tabindex=\"-1\"><header class=\"page-head\"><p class=\"eyebrow\">PROJECT TOOLS</p>");
    heading(
        &mut html,
        1,
        text(
            r.locale,
            "Find and inspect Project records",
            "프로젝트 기록 찾기 및 검사",
        ),
    );
    html.push_str(&format!("<p class=\"intro\">{}</p></header>",text(r.locale,"Find a request, decision or evidence record. Open it for permitted details and memory actions.","요청, 결정 또는 근거 기록을 찾습니다. 기록을 열어 허용된 상세 정보와 기억 관리 동작을 이용하세요.")));
    html.push_str(
        "<form class=\"inspection-query\" method=\"get\" action=\"/inspection\" role=\"search\">",
    );
    select(
        &mut html,
        "project_scope",
        text(r.locale, "Authorized Project", "승인된 프로젝트"),
        &page.project_id,
        std::iter::once((page.project_id.clone(), page.project_name.clone())),
    );
    // The Project select is informative, not an authority-bearing client selector.
    html = html.replace(
        "name=\"project_scope\"",
        "aria-describedby=\"project-scope\"",
    );
    select(
        &mut html,
        "class",
        text(
            r.locale,
            "What do you need to inspect?",
            "무엇을 검사하시겠습니까?",
        ),
        s.query.class.key(),
        InspectionClass::ALL
            .into_iter()
            .map(|c| (c.key().into(), class_name(c, r.locale).into())),
    );
    html.push_str(&format!("<label class=\"inspection-term\">{}<input type=\"search\" name=\"q\" value=\"{}\" maxlength=\"128\" aria-describedby=\"search-scope\" placeholder=\"{}\"></label>",text(r.locale,"Search visible summaries or exact identities","표시된 요약 또는 정확한 ID 검색"),escape(&s.query.term),text(r.locale,"e.g. retry policy","예: 재시도 정책")));
    html.push_str(&format!(
        "<details class=\"inspection-filters\"{}><summary>{}</summary><div>",
        if s.query.state.is_some() || s.query.role.is_some() {
            " open"
        } else {
            ""
        },
        text(
            r.locale,
            "Filter state, role and page size",
            "상태, 역할 및 페이지 크기 필터"
        )
    ));
    select(
        &mut html,
        "state",
        text(r.locale, "Recorded state", "기록된 상태"),
        s.query.state.as_deref().unwrap_or_default(),
        std::iter::once((
            String::new(),
            text(r.locale, "Any state", "모든 상태").into(),
        ))
        .chain(
            INSPECTION_STATES
                .iter()
                .map(|v| ((*v).into(), v.replace('_', " "))),
        ),
    );
    select(
        &mut html,
        "role",
        text(r.locale, "Statement role", "진술 역할"),
        s.query.role.as_deref().unwrap_or_default(),
        std::iter::once((
            String::new(),
            text(r.locale, "Any role", "모든 역할").into(),
        ))
        .chain(
            INSPECTION_ROLES
                .iter()
                .map(|v| ((*v).into(), v.replace('_', " "))),
        ),
    );
    select(
        &mut html,
        "limit",
        text(r.locale, "Rows per page", "페이지당 기록 수"),
        &s.query.limit.to_string(),
        [5, 10, 20]
            .into_iter()
            .map(|n| (n.to_string(), n.to_string())),
    );
    html.push_str("</div></details>");
    hidden(&mut html, "locale", locale_key(r.locale));
    hidden(&mut html, "language", &r.requested_language);
    html.push_str(&format!("<button type=\"submit\">{}</button></form><details class=\"inspection-help\"><summary>{}</summary><p class=\"meta\" id=\"project-scope\">{} <code>{}</code></p><p class=\"meta\" id=\"search-scope\">{}</p>",text(r.locale,"Find records","기록 찾기"),text(r.locale,"Read and search scope","읽기 및 검색 범위"),text(r.locale,"This viewer is bound to one Project. Open a separate viewer to inspect another authorized Project.","이 뷰어는 하나의 프로젝트에 연결됩니다. 다른 승인된 프로젝트는 별도 뷰어에서 검사합니다."),page.project_id,text(r.locale,"Literal search covers only displayed summaries (up to 240 characters) and IDs. Source bodies, withheld Candidate text and Derived bodies are excluded. Reads do not collect, renew retention or invoke a model.","일반 문자열로 표시된 요약(최대 240자)과 ID만 검색합니다. Source 본문, 비공개 Candidate 및 파생 본문은 제외됩니다. 조회는 수집, 보존 연장 또는 모델 호출을 하지 않습니다.")));
    html.push_str("</details>");
    let base = base_fields(r, s);
    if s.query.identity.is_some() {
        anchor(
            &mut html,
            &base,
            text(r.locale, "Back to results", "검색 결과로 돌아가기"),
        );
    }
    html.push_str(&format!("<section id=\"inspection-results\" class=\"inspection-results\" aria-labelledby=\"results-heading\"><h2 id=\"results-heading\">{} · {}</h2><p class=\"meta\">{} {} · {} {} · {} {}</p>",
        escape(&page.project_name),class_name(s.query.class,r.locale),page.permitted_match_count,text(r.locale,"permitted matches","허용된 검색 결과"),page.rows.len(),text(r.locale,"shown","표시"),page.omitted_match_count,text(r.locale,"remaining on later pages","이후 페이지에 남음")));
    if page.rows.is_empty() {
        empty_state(&mut html,text(r.locale,"No permitted records match these filters. Try another class or clear the filters. Withheld content is excluded from matching.","조건과 일치하는 허용된 기록이 없습니다. 다른 종류를 선택하거나 필터를 지워 보세요. 비공개 내용은 검색에 포함되지 않습니다."));
    }
    for row in &page.rows {
        let mut exact = base.clone();
        exact.push(("id", row.identity.clone()));
        html.push_str("<article class=\"inspection-record\">");
        html.push_str(&format!("<header><p class=\"record-state\">{} · {}</p><h3>{}</h3></header><p class=\"record-meta\">{} · <code>{}</code>{}</p>",escape(&row.role.replace('_'," ")),escape(&row.state.replace('_'," ")),escape(&row.summary),class_name(row.class,r.locale),row.identity,row.revision.map(|v|format!(" · r{v}")).unwrap_or_default()));
        if row.summary_omitted {
            html.push_str(&format!(
                "<p class=\"meta\">{}</p>",
                text(
                    r.locale,
                    "Summary shortened. Inspect the original field to read all retained text.",
                    "요약이 줄여 표시됩니다. 원래 필드를 열어 보존된 전체 내용을 읽을 수 있습니다."
                )
            ));
        }
        if s.query.identity.is_none() {
            anchor(
                &mut html,
                &exact,
                text(r.locale, "Inspect record", "기록 검사"),
            );
        }
        if row.class == InspectionClass::Work {
            html.push_str(&format!(
                "<a class=\"button\" href=\"/?view=work&amp;work={}&amp;locale={}&amp;language={}\">{}</a>",
                row.identity,locale_key(r.locale),escape(&percent_encode(&r.requested_language)),
                text(r.locale, "Read Work document", "작업 문서 읽기")
            ));
        } else if row.class == InspectionClass::Decision {
            html.push_str(&format!(
                "<a class=\"button\" href=\"/?view=decisions&amp;decision={}&amp;locale={}&amp;language={}\">{}</a>",
                row.identity,locale_key(r.locale),escape(&percent_encode(&r.requested_language)),
                text(r.locale, "Read Decision document", "결정 문서 읽기")
            ));
        }
        if s.query.identity.is_some() {
            html.push_str(&format!(
                "<p class=\"disclosure-state\">{}</p>",
                escape(&row.disclosure.replace('_', " "))
            ));
            if let Some(metadata) = row.metadata.as_object() {
                html.push_str("<dl class=\"inspection-metadata\">");
                for (key, value) in metadata {
                    let shown = if value.is_null() {
                        text(r.locale, "Not recorded", "기록 없음").into()
                    } else if key.ends_with("unix_micros") {
                        value
                            .as_i64()
                            .map(|t| observation_time(TimestampMicros::from_unix_micros(t)).1)
                            .unwrap_or_else(|| value.to_string())
                    } else {
                        value.as_str().unwrap_or_default().to_owned()
                    };
                    definition(
                        &mut html,
                        &key.replace("_unix_micros", "").replace('_', " "),
                        &shown,
                    );
                }
                html.push_str("</dl>");
            }
            heading(
                &mut html,
                4,
                text(r.locale, "Explicit relationships", "명시적인 관계"),
            );
            if row.relationships.is_empty() {
                empty_state(
                    &mut html,
                    text(
                        r.locale,
                        "No permitted explicit relationship is included.",
                        "허용된 명시적 관계가 포함되어 있지 않습니다.",
                    ),
                );
            }
            html.push_str("<ul class=\"inspection-relationships\">");
            for rel in &row.relationships {
                let fields = vec![
                    ("class", rel.class.key().into()),
                    ("id", rel.identity.clone()),
                    ("locale", locale_key(r.locale).into()),
                    ("language", r.requested_language.clone()),
                ];
                html.push_str(&format!(
                    "<li><span>{} · {}</span> ",
                    escape(&rel.relation.replace('_', " ")),
                    class_name(rel.class, r.locale)
                ));
                if let Some(revision) = rel.recorded_revision {
                    html.push_str(&format!(
                        "<span class=\"meta\">{} r{} · {} {}</span> ",
                        text(r.locale, "Recorded basis", "기록된 근거"),
                        revision,
                        text(r.locale, "current revision", "현재 리비전"),
                        rel.revision
                            .map(|r| r.to_string())
                            .unwrap_or_else(|| "unavailable".into())
                    ));
                }
                anchor(&mut html, &fields, &rel.identity);
                html.push_str("</li>");
            }
            html.push_str("</ul>");
            if row.relationships_omitted {
                empty_state(
                    &mut html,
                    text(
                        r.locale,
                        "Unavailable relationship targets are omitted.",
                        "사용할 수 없는 관계 대상은 생략됩니다.",
                    ),
                );
            }
            if let Some(offset) = row.next_relationship_offset {
                let mut next = exact.clone();
                next.push(("relationships", offset.to_string()));
                next.push(("basis", page.fingerprint.clone()));
                anchor(
                    &mut html,
                    &next,
                    text(r.locale, "More relationships", "관계 더 보기"),
                );
            }
            if !row.fields.is_empty() {
                html.push_str(
                    "<form class=\"inspection-field\" action=\"/inspection\" method=\"get\">",
                );
                for (k, v) in &exact {
                    hidden(&mut html, k, v);
                }
                hidden(
                    &mut html,
                    "revision",
                    &row.revision.unwrap_or_default().to_string(),
                );
                select(
                    &mut html,
                    "field",
                    text(r.locale, "Original field", "원래 필드"),
                    s.field.as_deref().unwrap_or_default(),
                    row.fields.iter().map(|v| (v.clone(), v.replace('_', " "))),
                );
                if row.class == InspectionClass::Source {
                    html.push_str(&format!("<label>{}<input name=\"work\" pattern=\"[0-9a-f]{{32}}\" value=\"{}\" placeholder=\"{}\"></label>",text(r.locale,"Work that retained the body (required for body access)","본문을 보존한 작업 ID (본문 조회에 필요)"),s.work.map(|id|id.to_string()).unwrap_or_default(),text(r.locale,"Exact Work ID","정확한 작업 ID")));
                }
                html.push_str(&format!(
                    "<button type=\"submit\">{}</button></form>",
                    text(r.locale, "Read selected field", "선택한 필드 읽기")
                ));
            } else {
                empty_state(&mut html,text(r.locale,"Content fields are withheld or unsupported here. This view exposes permitted metadata only.","본문 필드는 비공개이거나 여기서 지원하지 않습니다. 허용된 메타정보만 표시됩니다."));
            }
            if let Some(detail) = &detail {
                html.push_str("<section class=\"inspection-detail\" id=\"field-detail\" aria-labelledby=\"detail-heading\">");
                html.push_str(&format!(
                    "<h4 id=\"detail-heading\">{} · {}</h4><p class=\"record-state\">{} {}</p>",
                    escape(s.field.as_deref().unwrap_or_default()),
                    text(r.locale, "Original retained field", "보존된 원래 필드"),
                    escape(detail["state"].as_str().unwrap_or("unavailable")),
                    escape(
                        &detail["reason"]
                            .as_str()
                            .unwrap_or_default()
                            .replace('_', " ")
                    )
                ));
                if let Some(chunk) = detail["chunk"].as_str() {
                    html.push_str(&format!("<pre tabindex=\"0\">{}</pre>", escape(chunk)));
                }
                if let Some(offset) = detail["next_offset"].as_u64() {
                    let mut next = exact.clone();
                    next.extend([
                        ("field", s.field.clone().unwrap_or_default()),
                        ("revision", s.revision.unwrap_or_default().to_string()),
                        ("field_offset", offset.to_string()),
                        (
                            "field_basis",
                            detail["fingerprint"].as_str().unwrap_or_default().into(),
                        ),
                    ]);
                    if let Some(id) = s.work {
                        next.push(("work", id.to_string()));
                    }
                    anchor(
                        &mut html,
                        &next,
                        text(r.locale, "Read next field chunk", "필드 다음 부분 읽기"),
                    );
                }
                html.push_str(&format!(
                    "<details><summary>{}</summary><pre>{}</pre></details></section>",
                    text(
                        r.locale,
                        "Disclosure and Source state",
                        "공개 범위 및 Source 상태"
                    ),
                    escape(&detail["metadata"].to_string())
                ));
            }
            html.push_str(&format!(
                "<details class=\"audit\"><summary>{}</summary><pre>{}</pre></details>",
                text(
                    r.locale,
                    "Inspect permitted row envelope",
                    "허용된 행 데이터 검사"
                ),
                escape(&serde_json::to_string_pretty(row).unwrap_or_default())
            ));
        }
        html.push_str("</article>");
    }
    html.push_str("<nav class=\"inspection-paging\" aria-label=\"Record pages\">");
    if s.query.offset > 0 {
        anchor(
            &mut html,
            &base,
            text(
                r.locale,
                "Restart from first page",
                "첫 페이지부터 다시 조회",
            ),
        );
    }
    if let Some(offset) = page.next_offset {
        let mut next = base.clone();
        next.extend([
            ("offset", offset.to_string()),
            ("basis", page.fingerprint.clone()),
        ]);
        anchor(
            &mut html,
            &next,
            text(r.locale, "Next records", "다음 기록"),
        );
    }
    html.push_str("</nav></section>");
    let records = records_for_actions(&page.rows);
    if !records.is_empty() {
        render_mutation_records(&mut html, r, &records, token);
    }
    // Preserve the complete specialized Evidence journey behind deliberate disclosure.
    if matches!(
        r.view,
        crate::ViewerView::Tools {
            tool: crate::ViewerTool::Evidence
        }
    ) {
        html.push_str(&format!(
            "<details class=\"inspection-support\"><summary>{}</summary>",
            text(
                r.locale,
                "Repository inventory, history and diagnostics",
                "저장소 목록, 이력 및 진단"
            )
        ));
        if let Some(projection) = projection {
            let understanding = build_project_understanding(
                projection,
                UnderstandingBound {
                    max_items_per_section: 32,
                },
            );
            let health = adapter
                .operations()
                .health_stores_profiled(Some(r.project_id))
                .0;
            reading::diagnostics(&mut html, r, projection, &health);
            render_understanding_evidence(&mut html, r, &understanding);
            render_checkpoints(&mut html, r, projection);
            render_repository(&mut html, r, projection);
            render_overview(&mut html, r, projection);
        } else {
            empty_state(
                &mut html,
                text(
                    r.locale,
                    "Repository detail is unavailable. Open Analysis for recovery guidance.",
                    "저장소 상세를 사용할 수 없습니다. 분석 도구에서 복구 안내를 확인하세요.",
                ),
            );
        }
        html.push_str("</details>");
    }
    html.push_str("</main></body></html>");
    ViewerPage {
        verification_basis: Vec::new(),
        canonical_read_fingerprint: page.fingerprint,
        explanations: Vec::new(),
        project_id: r.project_id,
        html,
    }
}
