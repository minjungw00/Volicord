#[path = "presentation.rs"]
mod presentation;
#[path = "reading.rs"]
mod reading;
pub(crate) use presentation::locale_key;
use presentation::*;
use std::{
    error::Error as StdError,
    fmt,
    path::Path,
    time::{Duration, Instant},
};
use volicord_context::{
    ContextItemId, DecisionChoice, DecisionWorkScope, ProjectId, TimestampMicros,
    UserAcceptanceState, UserReviewState, VerificationState, WorkState,
};
use volicord_operations::{
    ConfirmationDecision, ConfirmationRequestId, ConfirmationResponse, GuardedEffectCategory,
    LocalOperations, ProjectProjectionProfile, PublicationOutcome,
};
use volicord_projections::{
    build_project_understanding, BriefDecision, BriefDecisionState, FixedLocale, ProjectProjection,
    ProjectUnderstanding, ProjectionIssueKind, UnderstandingBound, UnderstandingWork,
    UnderstandingWorkState,
};
use volicord_repository_intelligence::{Capability, CapabilityState, FreshnessState};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewerLocale {
    English,
    Korean,
}

impl ViewerLocale {
    const fn fixed(self) -> FixedLocale {
        match self {
            Self::English => FixedLocale::English,
            Self::Korean => FixedLocale::Korean,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewerRequest {
    pub project_id: ProjectId,
    pub locale: ViewerLocale,
    pub view: crate::ViewerView,
    /// Generated content language is carried as supplied and is not checked
    /// against the bundled fixed-text locales.
    pub requested_language: String,
    pub guarded_request: Option<ConfirmationRequestId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewerPage {
    pub verification_basis: Vec<volicord_projections::WorkVerificationBasis>,
    pub canonical_read_fingerprint: String,
    pub explanations: Vec<volicord_projections::ExplanationProvenance>,
    pub project_id: ProjectId,
    pub html: String,
}

/// Monotonic stage timings from the same path used to render a Viewer page.
///
/// The values are request-local diagnostics rather than a latency contract.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ViewerRenderProfile {
    pub projection: ProjectProjectionProfile,
    pub inspection_query: Duration,
    pub project_projection_passes: usize,
    pub understanding: Duration,
    pub health_read: Duration,
    pub health_analysis_snapshot_decodes: usize,
    pub privacy_read: Duration,
    pub document_preview: Duration,
    pub document_generations: usize,
    pub graph_diagnostics_requested: bool,
    pub guarded_read: Duration,
    pub html_render: Duration,
    pub total: Duration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ViewerRenderMode<'a> {
    Live { request_authenticity: &'a str },
    Snapshot { generated_at: TimestampMicros },
}

fn read_requirements(view: &crate::ViewerView) -> volicord_projections::ProjectionReadRequirements {
    volicord_projections::ProjectionReadRequirements {
        code: false,
        inspection: matches!(view, crate::ViewerView::Work { work: Some(_) }),
    }
}

#[derive(Debug)]
pub struct ViewerError {
    message: String,
    pub(crate) not_found: bool,
}

impl ViewerError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            not_found: false,
        }
    }
}

impl fmt::Display for ViewerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl StdError for ViewerError {}

pub struct ViewerAdapter {
    operations: LocalOperations,
}

impl ViewerAdapter {
    pub fn new(operations: LocalOperations) -> Self {
        crate::observation::prime_process_binding();
        Self { operations }
    }

    pub fn operations(&self) -> &LocalOperations {
        &self.operations
    }

    pub(crate) fn validate_view(
        &self,
        project: ProjectId,
        view: &crate::ViewerView,
    ) -> Result<(), ViewerError> {
        if matches!(
            view.selection(),
            volicord_projections::WorkSelector::ExactWork(_)
        ) {
            let (_projection, _) = self
                .operations
                .project_projection_read_profiled(
                    project,
                    view.selection(),
                    view.detail(),
                    read_requirements(view),
                )
                .map_err(|_| {
                    let mut e = ViewerError::new("selected identity is absent from this Project");
                    e.not_found = true;
                    e
                })?;
        }
        Ok(())
    }

    pub fn render(
        &self,
        request: &ViewerRequest,
        request_authenticity: &str,
    ) -> Result<ViewerPage, ViewerError> {
        self.render_profiled(request, request_authenticity)
            .map(|(page, _)| page)
    }

    pub fn render_profiled(
        &self,
        request: &ViewerRequest,
        request_authenticity: &str,
    ) -> Result<(ViewerPage, ViewerRenderProfile), ViewerError> {
        self.render_with_mode_profiled(
            request,
            ViewerRenderMode::Live {
                request_authenticity,
            },
        )
    }

    /// Renders the same Project projection as the live Viewer without any
    /// mutation transport, authenticity material, or live-server dependency.
    pub fn render_snapshot(
        &self,
        request: &ViewerRequest,
        generated_at: TimestampMicros,
    ) -> Result<ViewerPage, ViewerError> {
        self.render_snapshot_profiled(request, generated_at)
            .map(|(page, _)| page)
    }

    pub fn render_snapshot_profiled(
        &self,
        request: &ViewerRequest,
        generated_at: TimestampMicros,
    ) -> Result<(ViewerPage, ViewerRenderProfile), ViewerError> {
        if request.guarded_request.is_some() {
            return Err(ViewerError::new(
                "a static Viewer snapshot cannot render a Guarded response form",
            ));
        }
        let mut request = request.clone();
        request.view = crate::ViewerView::Overview;
        self.render_with_mode_profiled(&request, ViewerRenderMode::Snapshot { generated_at })
    }

    pub fn export_snapshot(
        &self,
        request: &ViewerRequest,
        destination: &Path,
    ) -> Result<PublicationOutcome, ViewerError> {
        self.export_snapshot_at(request, now()?, destination)
    }

    pub fn export_snapshot_at(
        &self,
        request: &ViewerRequest,
        generated_at: TimestampMicros,
        destination: &Path,
    ) -> Result<PublicationOutcome, ViewerError> {
        let page = self.render_snapshot(request, generated_at)?;
        self.operations
            .publish_viewer_snapshot(
                &page.html,
                destination,
                page.project_id,
                &page.canonical_read_fingerprint,
                &page.explanations,
                &page.verification_basis,
            )
            .map_err(|error| ViewerError::new(error.to_string()))
    }

    fn render_with_mode_profiled(
        &self,
        request: &ViewerRequest,
        mode: ViewerRenderMode<'_>,
    ) -> Result<(ViewerPage, ViewerRenderProfile), ViewerError> {
        let total_started = Instant::now();
        if let (
            Some(identity),
            ViewerRenderMode::Live {
                request_authenticity,
            },
        ) = (request.guarded_request, mode)
        {
            let candidate = self.operations.guarded_request(identity).map_err(|_| {
                let mut error = ViewerError::new("Guarded request unavailable");
                error.not_found = true;
                error
            })?;
            if candidate.project_id != request.project_id {
                let mut error = ViewerError::new("Guarded request unavailable");
                error.not_found = true;
                return Err(error);
            }
            let guarded_read = total_started.elapsed();
            let render_started = Instant::now();
            let mut html=format!("<!doctype html><html lang=\"{}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\"><title>Volicord confirmation</title>{STYLE}</head><body><header class=\"masthead\"><div class=\"shell\"><span class=\"brand\">Volicord</span></div></header><main class=\"shell\">", locale_key(request.locale));
            render_guarded(&mut html, request, &candidate, request_authenticity);
            html.push_str("</main></body></html>");
            return Ok((
                ViewerPage {
                    // Confirmation is not a canonical/document publication. Snapshot
                    // entry points reject guarded requests before rendering.
                    canonical_read_fingerprint: String::new(),
                    explanations: Vec::new(),
                    verification_basis: Vec::new(),
                    project_id: request.project_id,
                    html,
                },
                ViewerRenderProfile {
                    guarded_read,
                    html_render: render_started.elapsed(),
                    total: total_started.elapsed(),
                    ..Default::default()
                },
            ));
        }
        let snapshot = matches!(mode, ViewerRenderMode::Snapshot { .. });
        let (projection, projection_profile) = self
            .operations
            .project_projection_read_profiled(
                request.project_id,
                request.view.selection(),
                request.view.detail(),
                volicord_projections::ProjectionReadRequirements {
                    inspection: snapshot || read_requirements(&request.view).inspection,
                    ..read_requirements(&request.view)
                },
            )
            .map_err(|error| {
                let mut result = ViewerError::new(format!("cannot build Project view: {error}"));
                result.not_found = error.work_selection_cause().is_some();
                result
            })?;
        let understanding_started = Instant::now();
        let understanding = build_project_understanding(
            &projection,
            UnderstandingBound {
                max_items_per_section: 32,
            },
        );
        let understanding_duration = understanding_started.elapsed();
        let render_started = Instant::now();
        let mut html = String::new();
        html.push_str("<!doctype html><html lang=\"");
        html.push_str(locale_key(request.locale));
        html.push_str("\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\"><title>Volicord</title>");
        if !snapshot {
            match crate::observation::context(self.operations.layout().root(), request, &projection) {
                Ok(observation) => html.push_str(&format!(
                    "<meta name=\"volicord-observation\" content=\"{}\">",
                    escape(&observation.to_string())
                )),
                Err(_) => html.push_str("<meta name=\"volicord-observation-unavailable\" content=\"display-binding-unavailable\">"),
            }
        }
        html.push_str(STYLE);
        html.push_str(&format!(
            "</head><body data-view=\"{}\" data-viewer-mode=\"{}\">",
            request.view.key(),
            if matches!(mode, ViewerRenderMode::Live { .. }) {
                "live"
            } else {
                "snapshot"
            }
        ));
        html.push_str(&format!("<a class=\"skip skip-link\" href=\"#viewer-content\">{}</a><header class=\"masthead viewer-header\"><div class=\"shell\"><a class=\"brand\" href=\"{}\">Volicord</a>", text(request.locale, "Skip to content", "본문으로 건너뛰기"), if snapshot { "#overview".into() } else { format!("/?view=overview&locale={}&language={}", locale_key(request.locale), percent_encode(&request.requested_language)) }));
        if request.guarded_request.is_none() {
            reading::navigation(&mut html, request, snapshot);
        }
        html.push_str("</div></header><main class=\"shell viewer-shell\" id=\"viewer-content\" tabindex=\"-1\">");
        if request.guarded_request.is_none() {
            html.push_str(&format!(
                "<p class=\"surface-status\">{}</p>",
                if snapshot {
                    text(
                        request.locale,
                        "Read-only static publication",
                        "읽기 전용 정적 문서",
                    )
                } else {
                    text(
                        request.locale,
                        "Read-only reading surface",
                        "이 읽기 화면은 읽기 전용입니다",
                    )
                }
            ));
        }
        if snapshot {
            reading::snapshot(&mut html, request, &projection, &understanding);
            if let ViewerRenderMode::Snapshot { generated_at } = mode {
                html.push_str("<section class=\"snapshot-page\" id=\"snapshot-support\">");
                render_snapshot_basis(&mut html, request, &projection, generated_at);
                html.push_str("</section>");
            }
        } else {
            if request.guarded_request.is_none() {
                reading::surface(&mut html, request, &projection, &understanding);
            }
        }
        html.push_str("</main>");
        if !snapshot {
            html.push_str("<script data-viewer-reading>");
            html.push_str(crate::interaction::SCRIPT);
            html.push_str("</script>");
        }
        html.push_str("</body></html>");
        let html_render = render_started.elapsed();
        Ok((
            ViewerPage {
                verification_basis: volicord_projections::work_verification_publication_basis(
                    &projection,
                ),
                canonical_read_fingerprint: projection.canonical_read_fingerprint.clone(),
                explanations: projection
                    .work_history
                    .iter()
                    .chain(projection.selected_work.iter())
                    .chain(&projection.work_overview.current.items)
                    .chain(&projection.work_overview.completed.items)
                    .chain(&projection.work_overview.remaining.items)
                    .chain(&projection.work_overview.next_steps.items)
                    .flat_map(|w| {
                        w.reading
                            .explanations
                            .iter()
                            .chain(&w.reading.name_explanations)
                            .chain(&w.reading.summary_explanations)
                    })
                    .chain(&projection.repository_explanations)
                    .chain(
                        projection
                            .decision_catalog
                            .iter()
                            .chain(projection.selected_work_decisions.iter())
                            .filter(|d| {
                                projection
                                    .work_history
                                    .iter()
                                    .chain(projection.selected_work.iter())
                                    .any(|w| w.decision_ids.contains(&d.decision.decision_id))
                            })
                            .flat_map(|d| &d.decision.explanations),
                    )
                    .filter(|e| {
                        e.language == request.requested_language
                            && e.state == volicord_projections::ExplanationState::Current
                    })
                    .filter_map(|e| e.content.as_ref())
                    .map(Into::into)
                    .collect(),
                project_id: request.project_id,
                html,
            },
            ViewerRenderProfile {
                inspection_query: Duration::ZERO,
                projection: projection_profile,
                project_projection_passes: 1,
                understanding: understanding_duration,
                health_read: Duration::ZERO,
                health_analysis_snapshot_decodes: 0,
                privacy_read: Duration::ZERO,
                document_preview: Duration::ZERO,
                document_generations: 0,
                graph_diagnostics_requested: false,
                guarded_read: Duration::ZERO,
                html_render,
                total: total_started.elapsed(),
            },
        ))
    }

    pub fn confirm_guarded(
        &self,
        request: ConfirmationRequestId,
        revision: u64,
        fingerprint: &str,
        decision: ConfirmationDecision,
        session: String,
        user_turn: String,
    ) -> Result<ConfirmationResponse, ViewerError> {
        self.operations
            .record_confirmation(
                request,
                revision,
                fingerprint,
                decision,
                "local-viewer".into(),
                session,
                user_turn,
            )
            .map_err(|error| ViewerError::new(error.to_string()))
    }
}

fn render_snapshot_basis(
    html: &mut String,
    request: &ViewerRequest,
    projection: &ProjectProjection,
    generated_at: TimestampMicros,
) {
    let repository_snapshots = projection
        .resume
        .snapshots
        .iter()
        .map(|snapshot| snapshot.repository_snapshot.to_string())
        .collect::<Vec<_>>();
    let analysis_snapshots = projection
        .resume
        .snapshots
        .iter()
        .map(|snapshot| snapshot.analysis_snapshot.to_string())
        .collect::<Vec<_>>();
    let issues = projection
        .issues
        .iter()
        .map(|issue| {
            format!(
                "{}: {} (omitted={})",
                issue.affected_scope, issue.reason, issue.omitted_count
            )
        })
        .collect::<Vec<_>>();
    section_start(
        html,
        "snapshot-basis",
        text(request.locale, "Snapshot basis", "스냅샷 근거"),
    );
    html.push_str(&format!(
        "<details class=\"audit\"><summary>{}</summary><dl class=\"preview-meta\"><div><dt>{}</dt><dd><code>{}</code></dd></div><div><dt>{}</dt><dd>{}</dd></div><div><dt>{}</dt><dd>{}</dd></div><div><dt>{}</dt><dd>{}</dd></div><div><dt>{}</dt><dd>{}</dd></div><div><dt>{}</dt><dd>{}</dd></div></dl><p class=\"muted\">{}</p></details>",
        escape(text(request.locale, "Inspect Project, canonical, analysis, and freshness basis", "프로젝트, 정식 맥락, 분석 및 최신성 근거 검사")),
        escape(text(request.locale, "Project ID", "프로젝트 ID")),
        projection.overview.project_id,
        escape(text(request.locale, "Canonical revision", "정식 리비전")),
        projection.overview.canonical_revision,
        escape(text(request.locale, "Repository snapshots", "저장소 스냅샷")),
        escape(&bounded_names(&repository_snapshots, 8, request.locale)),
        escape(text(request.locale, "Analysis snapshots", "분석 스냅샷")),
        escape(&bounded_names(&analysis_snapshots, 8, request.locale)),
        escape(text(request.locale, "Generated at", "생성 시각")),
        generated_at.as_unix_micros(),
        escape(text(request.locale, "Projection degradation and omissions", "프로젝션 저하 및 생략")),
        escape(&bounded_names(&issues, 8, request.locale)),
        escape(text(request.locale, "The snapshot was generated locally and does not transmit or upload this artifact.", "스냅샷은 로컬에서 생성되며 이 산출물을 전송하거나 업로드하지 않습니다."))
    ));
    section_end(html);
}

fn render_guarded(
    html: &mut String,
    request: &ViewerRequest,
    candidate: &volicord_operations::GuardedEffectCandidate,
    request_authenticity: &str,
) {
    section_start(
        html,
        "guarded-confirmation",
        text(request.locale, "Guarded confirmation", "보호 확인"),
    );
    html.push_str(&format!(
        "<div class=\"guarded\"><p><code>{}</code> {} {} · {} <code>{}</code></p><p><strong>{}</strong> → <code>{}</code></p><p>{}</p><p>{}: {}</p><p>{} [{}] · {} {}</p></div>",
        candidate.confirmation_request_identity,
        escape(text(request.locale, "revision", "리비전")),
        candidate.request_revision,
        escape(text(request.locale, "fingerprint", "지문")),
        escape(&candidate.effect_fingerprint),
        escape(&candidate.exact_action),
        escape(&candidate.target),
        escape(&candidate.expected_effect),
        escape(guarded_category_label(candidate.risk.category, request.locale)),
        escape(&candidate.risk.concrete_consequence),
        escape(text(request.locale, "scope", "범위")),
        escape(&candidate.scope.join(", ")),
        escape(text(request.locale, "expires", "만료")),
        candidate.expires_at.as_unix_micros()
    ));
    empty_state(html, text(request.locale, "The response must carry this exact request identity, revision, and fingerprint; it is not general consent.", "응답은 이 정확한 요청 ID, 리비전 및 지문을 포함해야 하며 일반 동의가 아닙니다."));
    html.push_str(&format!("<form class=\"action-form\" method=\"post\" action=\"/guarded/confirm\"><fieldset><legend>{}</legend>", escape(text(request.locale, "Respond to this exact effect", "이 정확한 효과에 응답"))));
    hidden(
        html,
        "confirmation_request_id",
        &candidate.confirmation_request_identity.to_string(),
    );
    hidden(
        html,
        "request_revision",
        &candidate.request_revision.to_string(),
    );
    hidden(html, "effect_fingerprint", &candidate.effect_fingerprint);
    html.push_str(&format!(
        "<label>{} <textarea name=\"user_turn\" required></textarea></label>",
        escape(text(
            request.locale,
            "Current user turn",
            "현재 사용자 입력"
        ))
    ));
    render_view_fields(html, request, request_authenticity);
    html.push_str(&format!("<div class=\"button-row\"><button name=\"confirmation_decision\" value=\"confirm\" type=\"submit\">{}</button> <button name=\"confirmation_decision\" value=\"deny\" type=\"submit\">{}</button></div></fieldset></form>", escape(text(request.locale, "Confirm exact effect", "정확한 효과 확인")), escape(text(request.locale, "Deny", "거부"))));
    section_end(html);
}

fn bounded_names(values: &[String], limit: usize, locale: ViewerLocale) -> String {
    if values.is_empty() {
        return text(locale, "not reported", "보고되지 않음").to_owned();
    }
    let mut values = values.to_vec();
    values.sort();
    values.dedup();
    let mut rendered = values
        .iter()
        .take(limit)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let omitted = values.len().saturating_sub(limit);
    if omitted > 0 {
        rendered.push_str(&format!(
            "; {} {}",
            omitted,
            text(locale, "more omitted", "개 추가 생략")
        ));
    }
    rendered
}

const fn capability_label(capability: Capability, locale: ViewerLocale) -> &'static str {
    match capability {
        Capability::Inventory => text(locale, "inventory", "목록"),
        Capability::AgentAssisted => text(locale, "agent-assisted", "에이전트 보조"),
        Capability::Structural => text(locale, "structural", "구조 분석"),
        Capability::Semantic => text(locale, "semantic", "의미 분석"),
        Capability::Ecosystem => text(locale, "ecosystem", "생태계 분석"),
    }
}

const fn capability_state_label(state: CapabilityState, locale: ViewerLocale) -> &'static str {
    match state {
        CapabilityState::Available => text(locale, "available", "사용 가능"),
        CapabilityState::Unavailable => text(locale, "unavailable", "사용 불가"),
        CapabilityState::Unsupported => text(locale, "unsupported", "지원하지 않음"),
        CapabilityState::Partial => text(locale, "partial", "일부 가능"),
        CapabilityState::Failed => text(locale, "failed", "실패"),
        CapabilityState::Stale => text(locale, "stale", "오래됨"),
    }
}

const fn freshness_state_label(state: FreshnessState, locale: ViewerLocale) -> &'static str {
    match state {
        FreshnessState::Current => text(locale, "current", "현재"),
        FreshnessState::Stale => text(locale, "stale", "오래됨"),
        FreshnessState::Unknown => text(locale, "unknown", "알 수 없음"),
    }
}

const fn brief_decision_state_label(
    state: BriefDecisionState,
    locale: ViewerLocale,
) -> &'static str {
    match state {
        BriefDecisionState::Current => text(locale, "current", "현재 적용"),
        BriefDecisionState::StaleBasis => text(locale, "stale basis", "오래된 근거"),
        BriefDecisionState::ReviewRequired => text(locale, "review required", "검토 필요"),
        BriefDecisionState::Superseded => text(locale, "superseded", "대체됨"),
        BriefDecisionState::UnavailableBasis => text(locale, "basis unavailable", "근거 사용 불가"),
    }
}

const fn decision_scope_key(scope: DecisionWorkScope) -> &'static str {
    match scope {
        DecisionWorkScope::ProjectWide => "project-wide",
        DecisionWorkScope::WorkItem(_) => "work-item",
        DecisionWorkScope::Unresolved => "unresolved",
    }
}

const fn decision_scope_label(scope: DecisionWorkScope, locale: ViewerLocale) -> &'static str {
    match scope {
        DecisionWorkScope::ProjectWide => text(locale, "the whole Project", "프로젝트 전체"),
        DecisionWorkScope::WorkItem(_) => text(locale, "one Work Item", "특정 작업 항목"),
        DecisionWorkScope::Unresolved => text(
            locale,
            "scope not yet assigned",
            "아직 범위가 지정되지 않음",
        ),
    }
}

fn decision_choice_label(choice: &DecisionChoice, locale: ViewerLocale) -> String {
    match choice {
        DecisionChoice::Alternative { alternative_key } => format!(
            "{}: {}",
            text(locale, "Alternative", "대안"),
            alternative_key
        ),
        DecisionChoice::Delegation { delegate_to } => {
            format!(
                "{}: {}",
                text(locale, "Delegated to", "위임 대상"),
                delegate_to
            )
        }
    }
}

fn decision_choice_attribution(decision: &BriefDecision, locale: ViewerLocale) -> String {
    match decision.chosen_alternative_key.as_deref() {
        Some(key) => decision
            .displayed_alternatives
            .iter()
            .find(|alternative| alternative.key == key)
            .map_or_else(
                || decision_choice_label(&decision.choice, locale),
                |alternative| format!("{} [{}]", alternative.label, alternative.key),
            ),
        None => decision_choice_label(&decision.choice, locale),
    }
}

const fn work_state_label(state: WorkState, locale: ViewerLocale) -> &'static str {
    volicord_projections::work_state_label(state, locale.fixed())
}

const fn understanding_work_state_label(
    state: UnderstandingWorkState,
    locale: ViewerLocale,
) -> &'static str {
    volicord_projections::work_state_label_from_understanding(state, locale.fixed())
}

const fn understanding_work_state_key(state: UnderstandingWorkState) -> &'static str {
    match state {
        UnderstandingWorkState::Open => "open",
        UnderstandingWorkState::InProgress => "in-progress",
        UnderstandingWorkState::Paused => "paused",
        UnderstandingWorkState::Completed => "completed",
        UnderstandingWorkState::Abandoned => "abandoned",
        UnderstandingWorkState::Superseded => "superseded",
    }
}

const fn verification_state_label(state: VerificationState, locale: ViewerLocale) -> &'static str {
    volicord_projections::verification_state_label(state, locale.fixed())
}

const fn user_review_label(state: UserReviewState, locale: ViewerLocale) -> &'static str {
    volicord_projections::user_review_label(state, locale.fixed())
}

const fn user_acceptance_label(state: UserAcceptanceState, locale: ViewerLocale) -> &'static str {
    volicord_projections::user_acceptance_label(state, locale.fixed())
}

fn timestamp_label(timestamp: TimestampMicros, locale: ViewerLocale) -> String {
    let micros = timestamp.as_unix_micros();
    let seconds = micros.div_euclid(1_000_000);
    let remainder = micros.rem_euclid(1_000_000);
    format!(
        "{} {seconds}.{remainder:06} s",
        text(locale, "Unix time", "Unix 시각")
    )
}

const fn guarded_category_label(
    category: GuardedEffectCategory,
    locale: ViewerLocale,
) -> &'static str {
    match category {
        GuardedEffectCategory::DestructiveFileOrDataDeletion => text(
            locale,
            "destructive file or data deletion",
            "파괴적 파일 또는 데이터 삭제",
        ),
        GuardedEffectCategory::IrreversibleOrLargeScaleMigration => text(
            locale,
            "irreversible or large-scale migration",
            "되돌릴 수 없거나 대규모인 마이그레이션",
        ),
        GuardedEffectCategory::ExternalDeploymentOrPublicPublication => text(
            locale,
            "external deployment or public publication",
            "외부 배포 또는 공개 게시",
        ),
        GuardedEffectCategory::PaymentOrContinuingCost => {
            text(locale, "payment or continuing cost", "결제 또는 지속 비용")
        }
        GuardedEffectCategory::SecretOrCredentialAccessOrChange => text(
            locale,
            "secret or credential access or change",
            "비밀 또는 자격 증명 접근/변경",
        ),
        GuardedEffectCategory::PersonalDataOrSourceCodeExternalTransmission => text(
            locale,
            "external transmission of personal data or source code",
            "개인정보 또는 소스 코드 외부 전송",
        ),
        GuardedEffectCategory::ExternalMessageEmailOrIssue => text(
            locale,
            "external message, email, or issue",
            "외부 메시지, 이메일 또는 이슈",
        ),
        GuardedEffectCategory::ProductionDataChange => {
            text(locale, "production data change", "프로덕션 데이터 변경")
        }
        GuardedEffectCategory::PermissionAuthenticationOrSecuritySettingChange => text(
            locale,
            "permission, authentication, or security-setting change",
            "권한, 인증 또는 보안 설정 변경",
        ),
    }
}

fn now() -> Result<volicord_context::TimestampMicros, ViewerError> {
    use volicord_context::Clock;
    volicord_context::SystemClock
        .now()
        .map_err(|error| ViewerError::new(format!("system clock unavailable: {error}")))
}

fn render_view_fields(html: &mut String, request: &ViewerRequest, request_authenticity: &str) {
    hidden(html, "request_authenticity", request_authenticity);
    for (key, value) in request.view.fields() {
        hidden(html, key, &value);
    }
    hidden(html, "locale", locale_key(request.locale));
    hidden(html, "language", &request.requested_language);
    if let Some(identity) = request.guarded_request {
        hidden(html, "guarded", &identity.to_string());
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
