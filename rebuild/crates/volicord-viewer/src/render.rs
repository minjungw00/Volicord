use std::{error::Error as StdError, fmt, path::Path};
use volicord_context::{
    CanonicalRecordId, ContextItemCorrectionDraft, ContextItemId, CorrectionKind,
    DecisionCorrectionDraft, DecisionId, ProjectId, SourceId,
};
use volicord_operations::{
    CanonicalMutationOutcome, ConfirmationDecision, ConfirmationRequestId, ConfirmationResponse,
    HealthState, LocalOperations, PublicationOutcome,
};
use volicord_privacy::{ProviderConfigurationState, ProviderOptInState};
use volicord_projections::{
    CanonicalInspectionKind, DocumentKind, DocumentRequest, DocumentSet, FixedLocale,
    GeneratorIdentity, OutputFormat, ProjectProjection, ProjectionHealth, RequestedDestination,
};

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExplanationLevel {
    Overview,
    Working,
    Deep,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewerRequest {
    pub project_id: ProjectId,
    pub locale: ViewerLocale,
    pub explanation_level: ExplanationLevel,
    /// Generated content language is carried as supplied and is not checked
    /// against the bundled fixed-text locales.
    pub requested_language: String,
    pub guarded_request: Option<ConfirmationRequestId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewerPage {
    pub project_id: ProjectId,
    pub html: String,
}

#[derive(Debug)]
pub struct ViewerError {
    message: String,
}

impl ViewerError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
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
        Self { operations }
    }

    pub fn operations(&self) -> &LocalOperations {
        &self.operations
    }

    pub fn render(
        &self,
        request: &ViewerRequest,
        request_authenticity: &str,
    ) -> Result<ViewerPage, ViewerError> {
        let projection = self
            .operations
            .project_projection(request.project_id)
            .map_err(|error| ViewerError::new(format!("cannot build Project view: {error}")))?;
        let health = self.operations.health(Some(request.project_id));
        let privacy = self.operations.privacy_status(request.project_id).ok();
        let document_request = DocumentRequest {
            requested_language: request.requested_language.clone(),
            fixed_locale: request.locale.fixed(),
            generated_at: now()?,
            generator: GeneratorIdentity {
                generator: "volicord-local-viewer".into(),
                agent: None,
                model: None,
            },
            requested_destinations: Vec::new(),
        };
        let documents = self
            .operations
            .documents(request.project_id, &document_request)
            .map_err(|error| {
                ViewerError::new(format!("cannot generate document preview: {error}"))
            })?;
        let guarded = request
            .guarded_request
            .map(|identity| self.operations.guarded_request(identity))
            .transpose()
            .map_err(|error| {
                ViewerError::new(format!("cannot inspect Guarded request: {error}"))
            })?;

        let mut html = String::new();
        html.push_str("<!doctype html><html lang=\"");
        html.push_str(locale_key(request.locale));
        html.push_str("\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\"><title>Volicord</title>");
        html.push_str(STYLE);
        html.push_str(&format!(
            "</head><body data-explanation-level=\"{}\"><main>",
            level_key(request.explanation_level)
        ));
        heading(&mut html, 1, text(request.locale, "Project", "프로젝트"));
        html.push_str("<nav>");
        for (level, label) in [
            ("overview", text(request.locale, "Overview", "개요")),
            ("working", text(request.locale, "Working", "작업")),
            ("deep", text(request.locale, "Deep", "심층")),
        ] {
            html.push_str(&format!(
                "<a href=\"?level={level}&amp;locale={}&amp;language={}{}\">{}</a> ",
                locale_key(request.locale),
                percent_encode(&request.requested_language),
                request
                    .guarded_request
                    .map(|identity| format!("&amp;guarded={identity}"))
                    .unwrap_or_default(),
                escape(label)
            ));
        }
        html.push_str("</nav>");
        html.push_str(&format!(
            "<p><strong>{}:</strong> {}</p>",
            escape(text(request.locale, "Explanation level", "설명 수준")),
            escape(level_key(request.explanation_level))
        ));
        render_status(&mut html, request, &projection, &health);
        render_overview(&mut html, request, &projection);
        render_repository(&mut html, request, &projection);
        render_decisions(&mut html, request, &projection);
        render_checkpoints(&mut html, request, &projection);
        render_candidates(&mut html, request, &projection);
        render_canonical(&mut html, request, &projection);
        render_privacy(&mut html, request, privacy.as_ref());
        render_documents(&mut html, request, &documents, request_authenticity);
        render_mutation_controls(&mut html, request, &projection, request_authenticity);
        if let Some(candidate) = guarded.as_ref() {
            render_guarded(&mut html, request, candidate, request_authenticity);
        }
        html.push_str("</main></body></html>");
        Ok(ViewerPage {
            project_id: request.project_id,
            html,
        })
    }

    pub fn correct_context(
        &self,
        project_id: ProjectId,
        item_id: ContextItemId,
        expected_revision: u64,
        corrected_statement: String,
        authorization: SourceId,
    ) -> Result<CanonicalMutationOutcome, ViewerError> {
        self.operations
            .correct_context_item(
                project_id,
                item_id,
                ContextItemCorrectionDraft {
                    expected_revision,
                    corrected_statement,
                    kind: CorrectionKind::Expression,
                    user_authorization_source_id: authorization,
                },
            )
            .map_err(|error| ViewerError::new(error.to_string()))
    }

    pub fn correct_decision(
        &self,
        project_id: ProjectId,
        decision_id: DecisionId,
        expected_revision: u64,
        rationale: String,
        authorization: SourceId,
    ) -> Result<CanonicalMutationOutcome, ViewerError> {
        self.operations
            .correct_decision(
                project_id,
                decision_id,
                DecisionCorrectionDraft {
                    expected_revision,
                    corrected_user_rationale: Some(rationale),
                    kind: CorrectionKind::Expression,
                    user_authorization_source_id: authorization,
                },
            )
            .map_err(|error| ViewerError::new(error.to_string()))
    }

    pub fn supersede_decision(
        &self,
        project_id: ProjectId,
        previous: DecisionId,
        user_source: SourceId,
        alternative: String,
        rationale: Option<String>,
    ) -> Result<CanonicalMutationOutcome, ViewerError> {
        self.operations
            .supersede_decision_choice(project_id, previous, user_source, alternative, rationale)
            .map_err(|error| ViewerError::new(error.to_string()))
    }

    pub fn forget(
        &self,
        project_id: ProjectId,
        record: CanonicalRecordId,
        authorization: SourceId,
    ) -> Result<CanonicalMutationOutcome, ViewerError> {
        self.operations
            .forget_record(project_id, record, authorization)
            .map_err(|error| ViewerError::new(error.to_string()))
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

    pub fn export_document(
        &self,
        project_id: ProjectId,
        kind: DocumentKind,
        format: OutputFormat,
        destination: &Path,
        requested_language: String,
        locale: ViewerLocale,
    ) -> Result<PublicationOutcome, ViewerError> {
        let request = DocumentRequest {
            requested_language,
            fixed_locale: locale.fixed(),
            generated_at: now()?,
            generator: GeneratorIdentity {
                generator: "volicord-local-viewer".into(),
                agent: None,
                model: None,
            },
            requested_destinations: vec![RequestedDestination {
                document_kind: kind,
                output_format: format,
                path: destination.display().to_string(),
            }],
        };
        let documents = self
            .operations
            .documents(project_id, &request)
            .map_err(|error| ViewerError::new(error.to_string()))?;
        let document = select_document(&documents, kind);
        self.operations
            .publish_document(document, format, destination)
            .map_err(|error| ViewerError::new(error.to_string()))
    }
}

fn render_status(
    html: &mut String,
    request: &ViewerRequest,
    projection: &ProjectProjection,
    health: &volicord_operations::HealthReport,
) {
    heading(
        html,
        2,
        text(request.locale, "Operation status", "작동 상태"),
    );
    let state = match health.state {
        HealthState::Healthy => text(request.locale, "healthy", "정상"),
        HealthState::Degraded => text(request.locale, "degraded", "저하됨"),
        HealthState::Failed => text(request.locale, "failed", "실패"),
    };
    html.push_str(&format!(
        "<p class=\"state {:?}\">{}: <strong>{}</strong>; projection: <strong>{:?}</strong></p>",
        health.state,
        escape(text(request.locale, "Runtime", "런타임")),
        escape(state),
        projection.health
    ));
    for issue in &health.issues {
        item(
            html,
            &format!("{:?} · {} · {}", issue.kind, issue.scope, issue.detail),
        );
    }
    for issue in &projection.issues {
        item(
            html,
            &format!(
                "{:?} · {} · {}",
                issue.kind, issue.affected_scope, issue.reason
            ),
        );
    }
}

fn render_overview(html: &mut String, request: &ViewerRequest, projection: &ProjectProjection) {
    heading(
        html,
        2,
        text(request.locale, "Project overview", "프로젝트 개요"),
    );
    let overview = &projection.overview;
    html.push_str(&format!(
        "<p><strong>{}</strong> <code>{}</code> · revision {} · {:?}</p>",
        escape(&overview.project_name),
        overview.project_id,
        overview.canonical_revision,
        overview.health
    ));
    for goal in &overview.current_goals {
        item(html, goal);
    }
    html.push_str(&format!(
        "<p>{}: {} · {}: {} · {}: {}</p>",
        escape(text(request.locale, "Active decisions", "활성 결정")),
        overview.active_decision_count,
        escape(text(request.locale, "Open questions", "열린 질문")),
        overview.open_question_count,
        escape(text(request.locale, "Superseded decisions", "대체된 결정")),
        overview.superseded_decision_count
    ));
    heading(html, 3, text(request.locale, "Resume brief", "재개 요약"));
    for goal in &projection.resume.goals_and_why {
        item(html, &goal.statement);
    }
    if let Some(step) = &projection.resume.next_meaningful_step {
        html.push_str(&format!(
            "<p><strong>{}:</strong> {}</p>",
            escape(text(request.locale, "Next step", "다음 단계")),
            escape(step)
        ));
    }
    html.push_str(&format!(
        "<p>{}: {}</p>",
        escape(text(request.locale, "Omitted", "생략")),
        projection.resume.omitted_count
    ));
}

fn render_repository(html: &mut String, request: &ViewerRequest, projection: &ProjectProjection) {
    heading(
        html,
        2,
        text(request.locale, "Repository Map", "저장소 지도"),
    );
    let map = &projection.repository_map;
    html.push_str(&format!(
        "<p>{} entities · {} relations · {} gaps · {:?}</p>",
        map.entities.len(),
        map.relations.len(),
        map.gaps.len(),
        map.health
    ));
    if request.explanation_level != ExplanationLevel::Overview {
        for entity in visible(&map.entities, request.explanation_level) {
            item(
                html,
                &format!(
                    "{:?} · {:?} · {} · source {} · {:?}",
                    entity.language,
                    entity.kind,
                    entity.display_name,
                    entity.source_id,
                    entity.freshness.state
                ),
            );
        }
    }
    if request.explanation_level == ExplanationLevel::Deep {
        for relation in &map.relations {
            item(
                html,
                &format!(
                    "{:?} · {} · {} → {} · {:?}",
                    relation.class,
                    relation.kind,
                    relation.source_entity,
                    relation.target_entity.as_deref().unwrap_or("unresolved"),
                    relation.freshness.state
                ),
            );
        }
    }
    for gap in &map.gaps {
        item(
            html,
            &format!(
                "{:?} · {:?} · {} · {} · remainder: {}",
                gap.state,
                gap.capability,
                gap.area,
                gap.reason,
                gap.usable_remainder.as_deref().unwrap_or("none")
            ),
        );
    }
}

fn render_decisions(html: &mut String, request: &ViewerRequest, projection: &ProjectProjection) {
    heading(
        html,
        2,
        text(
            request.locale,
            "Decision trail / Context / Code",
            "결정 이력 / 맥락 / 코드",
        ),
    );
    for link in visible(&projection.decision_context_code, request.explanation_level) {
        item(
            html,
            &format!(
                "{} r{} · {:?} · paths [{}] · code [{}] · uncertainty [{}]",
                link.decision_id,
                link.decision_revision,
                link.decision_state,
                link.declared_paths.join(", "),
                link.related_code_entities.join(", "),
                link.missing_or_uncertain_links.join(", ")
            ),
        );
    }
}

fn render_checkpoints(html: &mut String, request: &ViewerRequest, projection: &ProjectProjection) {
    heading(
        html,
        2,
        text(request.locale, "Checkpoint timeline", "체크포인트 타임라인"),
    );
    for entry in visible(&projection.checkpoint_timeline, request.explanation_level) {
        item(
            html,
            &format!(
                "{} · {:?} · work {:?} · verification {:?} · next {}",
                entry.checkpoint.id,
                entry.checkpoint.kind,
                entry.work_state,
                entry
                    .verification
                    .iter()
                    .map(|fact| fact.state)
                    .collect::<Vec<_>>(),
                entry.checkpoint.next_step
            ),
        );
    }
}

fn render_candidates(html: &mut String, request: &ViewerRequest, projection: &ProjectProjection) {
    heading(
        html,
        2,
        text(request.locale, "Candidate inspection", "후보 검사"),
    );
    for candidate in visible(&projection.candidate_inspection, request.explanation_level) {
        item(
            html,
            &format!(
                "{} · {:?} · exists {} · health {:?} · disposition {:?} · opt-out {:?} · {}",
                candidate.candidate_id,
                candidate.kind,
                candidate.exists,
                candidate.health,
                candidate.promotion_disposition,
                candidate.current_applicable_opt_out,
                candidate
                    .bounded_summary
                    .as_deref()
                    .unwrap_or("content unavailable")
            ),
        );
    }
}

fn render_canonical(html: &mut String, request: &ViewerRequest, projection: &ProjectProjection) {
    heading(
        html,
        2,
        text(request.locale, "Canonical context", "정식 맥락"),
    );
    for record in visible(&projection.canonical_inspection, request.explanation_level) {
        item(
            html,
            &format!(
                "{:?} · {} r{} · {} · {}",
                record.kind,
                record.identity,
                record.revision,
                record.lifecycle_state,
                record.summary
            ),
        );
    }
}

fn render_privacy(
    html: &mut String,
    request: &ViewerRequest,
    privacy: Option<&volicord_privacy::ProjectPrivacyInspection>,
) {
    heading(
        html,
        2,
        text(request.locale, "Privacy and provider", "개인정보 및 공급자"),
    );
    match privacy {
        Some(value) => {
            let configuration = match value.configuration_state {
                ProviderConfigurationState::NeverEnabled => "never-enabled",
                ProviderConfigurationState::Enabled => "enabled",
                ProviderConfigurationState::Disabled => "disabled",
                ProviderConfigurationState::Revoked => "revoked",
            };
            html.push_str(&format!("<p>{}</p>", escape(configuration)));
            if let Some(event) = &value.current_opt_in {
                let state = match event.state {
                    ProviderOptInState::Enabled => "enabled",
                    ProviderOptInState::Disabled => "disabled",
                    ProviderOptInState::Revoked => "revoked",
                };
                item(
                    html,
                    &format!(
                        "{} · {} / {} · scope [{}] · requests {} · derived {}",
                        state,
                        event.policy.provider,
                        event.policy.model,
                        event.policy.allowed_source_scopes.join(", "),
                        value.requests.len(),
                        value.managed_derived.len()
                    ),
                );
            } else {
                item(
                    html,
                    text(
                        request.locale,
                        "Local-only mode; no background provider consent",
                        "로컬 전용 모드; 백그라운드 공급자 동의 없음",
                    ),
                );
            }
        }
        None => item(
            html,
            text(
                request.locale,
                "Privacy state unavailable; canonical views remain available",
                "개인정보 상태를 사용할 수 없음; 정식 맥락 보기는 계속 가능",
            ),
        ),
    }
}

fn render_documents(
    html: &mut String,
    request: &ViewerRequest,
    documents: &DocumentSet,
    request_authenticity: &str,
) {
    heading(
        html,
        2,
        text(
            request.locale,
            "Document preview / export",
            "문서 미리보기 / 내보내기",
        ),
    );
    for kind in DocumentKind::ALL {
        let document = select_document(documents, kind);
        html.push_str(&format!(
            "<details><summary>{}</summary><p>language: {} · canonical revision {} · snapshots {} · gaps {} · omissions {}</p><pre>{}</pre></details>",
            escape(document.metadata.document_kind.slug()),
            escape(&document.metadata.requested_language),
            document.metadata.canonical_revision,
            document.metadata.analysis_snapshots.len(),
            document.metadata.capability_gaps.len(),
            document.metadata.omissions.len(),
            escape(&document.markdown.content)
        ));
    }
    html.push_str(&format!(
        "<form method=\"post\" action=\"/documents/export\"><label>{} <select name=\"kind\">",
        escape(text(request.locale, "Document", "문서"))
    ));
    for kind in DocumentKind::ALL {
        html.push_str(&format!(
            "<option value=\"{}\">{}</option>",
            escape(kind.slug()),
            escape(kind.slug())
        ));
    }
    html.push_str(&format!("</select></label> <label>{} <select name=\"format\"><option value=\"markdown\">Markdown</option><option value=\"html\">HTML</option></select></label> <label>{} <input name=\"destination\" required></label>", escape(text(request.locale, "Format", "형식")), escape(text(request.locale, "Absolute destination", "절대 대상 경로"))));
    render_view_fields(html, request, request_authenticity);
    html.push_str(&format!(
        "<button type=\"submit\">{}</button></form>",
        escape(text(request.locale, "Export", "내보내기"))
    ));
    item(html, text(request.locale, "Export writes only to an explicit absolute destination and never adopts the document automatically", "내보내기는 명시한 절대 경로에만 쓰며 문서를 자동 채택하지 않음"));
}

fn render_mutation_controls(
    html: &mut String,
    request: &ViewerRequest,
    projection: &ProjectProjection,
    request_authenticity: &str,
) {
    heading(html, 2, text(request.locale, "Memory actions", "기억 작업"));
    item(html, text(request.locale, "Correction, supersession, forgetting, provider changes, document publication, and Guarded responses are submitted to Local Operations; this viewer has no write store", "수정, 대체, 삭제, 공급자 변경, 문서 게시 및 보호 응답은 로컬 작업 계층에 제출됨; 이 뷰어에는 쓰기 저장소가 없음"));
    for record in &projection.canonical_inspection {
        match record.kind {
            CanonicalInspectionKind::ContextItem => {
                html.push_str(&format!("<details><summary>{} <code>{}</code></summary><form method=\"post\" action=\"/memory/context/correct\">", escape(text(request.locale, "Correct Context Item", "맥락 항목 수정")), escape(&record.identity)));
                hidden(html, "record_id", &record.identity);
                hidden(html, "expected_revision", &record.revision.to_string());
                html.push_str(&format!("<label>{} <textarea name=\"corrected_text\" required></textarea></label><label>{} <textarea name=\"user_turn\" required></textarea></label>", escape(text(request.locale, "Corrected statement", "수정한 진술")), escape(text(request.locale, "Current user turn", "현재 사용자 입력"))));
                render_view_fields(html, request, request_authenticity);
                html.push_str(&format!(
                    "<button type=\"submit\">{}</button></form></details>",
                    escape(text(request.locale, "Correct", "수정"))
                ));
            }
            CanonicalInspectionKind::Decision => {
                html.push_str(&format!("<details><summary>{} <code>{}</code></summary><form method=\"post\" action=\"/memory/decision/correct\">", escape(text(request.locale, "Correct or supersede Decision", "결정 수정 또는 대체")), escape(&record.identity)));
                hidden(html, "record_id", &record.identity);
                hidden(html, "expected_revision", &record.revision.to_string());
                html.push_str(&format!("<label>{} <textarea name=\"corrected_text\" required></textarea></label><label>{} <textarea name=\"user_turn\" required></textarea></label>", escape(text(request.locale, "Corrected rationale", "수정한 근거")), escape(text(request.locale, "Current user turn", "현재 사용자 입력"))));
                render_view_fields(html, request, request_authenticity);
                html.push_str(&format!("<button type=\"submit\">{}</button></form><form method=\"post\" action=\"/memory/decision/supersede\">", escape(text(request.locale, "Correct rationale", "근거 수정"))));
                hidden(html, "record_id", &record.identity);
                html.push_str(&format!("<label>{} <input name=\"alternative\" required></label><label>{} <textarea name=\"rationale\"></textarea></label><label>{} <textarea name=\"user_turn\" required></textarea></label>", escape(text(request.locale, "New displayed alternative key", "새 표시 대안 키")), escape(text(request.locale, "Rationale", "근거")), escape(text(request.locale, "Current user turn", "현재 사용자 입력"))));
                render_view_fields(html, request, request_authenticity);
                html.push_str(&format!(
                    "<button type=\"submit\">{}</button></form></details>",
                    escape(text(request.locale, "Supersede", "대체"))
                ));
            }
            _ => {}
        }
        if let Some(kind) = forgettable_kind(record.kind) {
            html.push_str(&format!("<details><summary>{} {:?} <code>{}</code></summary><form method=\"post\" action=\"/memory/forget\">", escape(text(request.locale, "Forget", "삭제")), record.kind, escape(&record.identity)));
            hidden(html, "record_kind", kind);
            hidden(html, "record_id", &record.identity);
            html.push_str(&format!(
                "<label>{} <textarea name=\"user_turn\" required></textarea></label>",
                escape(text(
                    request.locale,
                    "Current user turn",
                    "현재 사용자 입력"
                ))
            ));
            render_view_fields(html, request, request_authenticity);
            html.push_str(&format!(
                "<button type=\"submit\">{}</button></form></details>",
                escape(text(request.locale, "Forget this record", "이 기록 삭제"))
            ));
        }
    }
}

fn render_guarded(
    html: &mut String,
    request: &ViewerRequest,
    candidate: &volicord_operations::GuardedEffectCandidate,
    request_authenticity: &str,
) {
    heading(
        html,
        2,
        text(request.locale, "Guarded confirmation", "보호 확인"),
    );
    html.push_str(&format!(
        "<div class=\"guarded\"><p><code>{}</code> revision {} fingerprint <code>{}</code></p><p><strong>{}</strong> → <code>{}</code></p><p>{}</p><p>{:?}: {}</p><p>scope [{}] · expires {}</p></div>",
        candidate.confirmation_request_identity,
        candidate.request_revision,
        escape(&candidate.effect_fingerprint),
        escape(&candidate.exact_action),
        escape(&candidate.target),
        escape(&candidate.expected_effect),
        candidate.risk.category,
        escape(&candidate.risk.concrete_consequence),
        escape(&candidate.scope.join(", ")),
        candidate.expires_at.as_unix_micros()
    ));
    item(html, text(request.locale, "The response must carry this exact request identity, revision, and fingerprint; it is not general consent", "응답은 이 정확한 요청 ID, 리비전 및 지문을 포함해야 하며 일반 동의가 아님"));
    html.push_str("<form method=\"post\" action=\"/guarded/confirm\">");
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
    html.push_str(&format!("<button name=\"decision\" value=\"confirm\" type=\"submit\">{}</button> <button name=\"decision\" value=\"deny\" type=\"submit\">{}</button></form>", escape(text(request.locale, "Confirm exact effect", "정확한 효과 확인")), escape(text(request.locale, "Deny", "거부"))));
}

fn visible<T>(values: &[T], level: ExplanationLevel) -> &[T] {
    let limit = match level {
        ExplanationLevel::Overview => 3,
        ExplanationLevel::Working => 20,
        ExplanationLevel::Deep => values.len(),
    };
    &values[..values.len().min(limit)]
}

fn select_document(
    set: &DocumentSet,
    kind: DocumentKind,
) -> &volicord_projections::GeneratedDocument {
    match kind {
        DocumentKind::ProjectArchitectureGuide => &set.project_architecture_guide,
        DocumentKind::DecisionReport => &set.decision_report,
        DocumentKind::ImplementationPlan => &set.implementation_plan,
        DocumentKind::HandoffResume => &set.handoff_resume,
    }
}

fn now() -> Result<volicord_context::TimestampMicros, ViewerError> {
    use volicord_context::Clock;
    volicord_context::SystemClock
        .now()
        .map_err(|error| ViewerError::new(format!("system clock unavailable: {error}")))
}

fn heading(html: &mut String, level: u8, value: &str) {
    html.push_str(&format!("<h{level}>{}</h{level}>", escape(value)));
}

fn item(html: &mut String, value: &str) {
    html.push_str(&format!("<div class=\"item\">{}</div>", escape(value)));
}

fn text<'a>(locale: ViewerLocale, english: &'a str, korean: &'a str) -> &'a str {
    match locale {
        ViewerLocale::English => english,
        ViewerLocale::Korean => korean,
    }
}

fn locale_key(locale: ViewerLocale) -> &'static str {
    match locale {
        ViewerLocale::English => "en",
        ViewerLocale::Korean => "ko",
    }
}

fn level_key(level: ExplanationLevel) -> &'static str {
    match level {
        ExplanationLevel::Overview => "overview",
        ExplanationLevel::Working => "working",
        ExplanationLevel::Deep => "deep",
    }
}

fn forgettable_kind(kind: CanonicalInspectionKind) -> Option<&'static str> {
    match kind {
        CanonicalInspectionKind::Project => None,
        CanonicalInspectionKind::Source => Some("source"),
        CanonicalInspectionKind::Question => Some("question"),
        CanonicalInspectionKind::Decision => Some("decision"),
        CanonicalInspectionKind::ContextItem => Some("context_item"),
        CanonicalInspectionKind::Checkpoint => Some("checkpoint"),
    }
}

fn render_view_fields(html: &mut String, request: &ViewerRequest, request_authenticity: &str) {
    hidden(html, "request_authenticity", request_authenticity);
    hidden(html, "level", level_key(request.explanation_level));
    hidden(html, "locale", locale_key(request.locale));
    hidden(html, "language", &request.requested_language);
    if let Some(identity) = request.guarded_request {
        hidden(html, "guarded", &identity.to_string());
    }
}

fn hidden(html: &mut String, name: &str, value: &str) {
    html.push_str(&format!(
        "<input type=\"hidden\" name=\"{}\" value=\"{}\">",
        escape(name),
        escape(value)
    ));
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

const STYLE: &str = r#"<style>
:root{color-scheme:light dark;font-family:system-ui,sans-serif}body{margin:0;background:#111827;color:#e5e7eb}main{max-width:1100px;margin:auto;padding:2rem}h1,h2,h3{color:#f9fafb}h2{border-top:1px solid #374151;padding-top:1rem}a{color:#93c5fd}.item,details,.state,.guarded{padding:.65rem .8rem;margin:.4rem 0;background:#1f2937;border-radius:.45rem}.Degraded,.degraded{border-left:4px solid #f59e0b}.Failed,.failed{border-left:4px solid #ef4444}.guarded{border:2px solid #f59e0b}code,pre{white-space:pre-wrap;overflow-wrap:anywhere}pre{max-height:32rem;overflow:auto}form{display:grid;gap:.5rem;margin:.6rem 0}label{display:grid;gap:.25rem}textarea,input,select,button{font:inherit;padding:.45rem}button{width:max-content}
</style>"#;

#[allow(dead_code)]
fn _projection_health_is_explicit(value: ProjectionHealth) -> &'static str {
    match value {
        ProjectionHealth::Complete => "complete",
        ProjectionHealth::Partial => "partial",
        ProjectionHealth::Degraded => "degraded",
    }
}
