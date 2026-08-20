use std::{collections::BTreeMap, error::Error as StdError, fmt, path::Path};
use volicord_context::{
    CanonicalRecordId, CheckpointKind, ContextItemCorrectionDraft, ContextItemId, CorrectionKind,
    DecisionChoice, DecisionCorrectionDraft, DecisionId, ProjectId, SourceId, SourcePayload,
    TimestampMicros, UserAcceptanceState, UserReviewState, VerificationState, WorkState,
};
use volicord_inquiry::{CandidateDisposition, CandidateKind};
use volicord_operations::{
    CanonicalMutationOutcome, ConfirmationDecision, ConfirmationRequestId, ConfirmationResponse,
    ForgettingOutcome, GuardedEffectCategory, HealthIssueKind, HealthState, LocalOperations,
    PublicationOutcome,
};
use volicord_privacy::{ProviderConfigurationState, ProviderOptInState};
use volicord_projections::{
    BriefDecisionState, CanonicalInspectionKind, ClaimClass, DocumentKind, DocumentRequest,
    DocumentSet, FixedLocale, GeneratorIdentity, InspectionHealth, MapRelationClass, OutputFormat,
    ProjectProjection, ProjectionHealth, ProjectionIssueKind, RequestedDestination,
};
use volicord_repository_intelligence::{
    Capability, CapabilityState, CodeEntityKind, FreshnessState, Language,
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
        html.push_str(&format!(
            "<nav aria-label=\"{}\"><ul class=\"level-nav\">",
            escape(text(request.locale, "Explanation level", "설명 수준"))
        ));
        for (level, label) in [
            ("overview", text(request.locale, "Overview", "개요")),
            ("working", text(request.locale, "Working", "작업")),
            ("deep", text(request.locale, "Deep", "심층")),
        ] {
            let current = if level == level_key(request.explanation_level) {
                " aria-current=\"page\""
            } else {
                ""
            };
            html.push_str(&format!(
                "<li><a{current} href=\"?level={level}&amp;locale={}&amp;language={}{}\">{}</a></li>",
                locale_key(request.locale),
                percent_encode(&request.requested_language),
                request
                    .guarded_request
                    .map(|identity| format!("&amp;guarded={identity}"))
                    .unwrap_or_default(),
                escape(label)
            ));
        }
        html.push_str("</ul></nav>");
        html.push_str(&format!(
            "<p><strong>{}:</strong> {}</p>",
            escape(text(request.locale, "Explanation level", "설명 수준")),
            escape(explanation_level_label(
                request.explanation_level,
                request.locale
            ))
        ));
        render_overview(&mut html, request, &projection);
        render_decisions(&mut html, request, &projection);
        render_status(&mut html, request, &projection, &health);
        if let Some(candidate) = guarded.as_ref() {
            render_guarded(&mut html, request, candidate, request_authenticity);
        }
        render_checkpoints(&mut html, request, &projection);
        render_repository(&mut html, request, &projection);
        render_candidates(&mut html, request, &projection);
        render_canonical(&mut html, request, &projection);
        render_privacy(&mut html, request, privacy.as_ref());
        render_documents(&mut html, request, &documents, request_authenticity);
        render_mutation_controls(&mut html, request, &projection, request_authenticity);
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
    ) -> Result<ForgettingOutcome, ViewerError> {
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
    section_start(
        html,
        "health",
        text(
            request.locale,
            "Health and usable capability",
            "상태 및 사용 가능한 기능",
        ),
    );
    html.push_str(&format!(
        "<p class=\"state\" data-state=\"{}\"><strong>{}:</strong> {} · <strong>{}:</strong> {}</p>",
        health_state_key(health.state),
        escape(text(request.locale, "Runtime", "런타임")),
        escape(health_state_label(health.state, request.locale)),
        escape(text(request.locale, "Projection", "프로젝션")),
        escape(projection_health_label(projection.health, request.locale))
    ));
    html.push_str("<ul class=\"status-summary\">");
    status_item(
        html,
        request,
        text(request.locale, "Canonical memory", "정식 기억"),
        health.canonical_available,
    );
    status_item(
        html,
        request,
        text(request.locale, "Candidate inspection", "후보 검사"),
        health.candidate_available,
    );
    status_item(
        html,
        request,
        text(request.locale, "Privacy controls", "개인정보 제어"),
        health.privacy_available,
    );
    if let Some(available) = health.repository_available {
        status_item(
            html,
            request,
            text(request.locale, "Repository analysis", "저장소 분석"),
            available,
        );
    }
    html.push_str("</ul>");

    let groups = degradation_groups(projection, request.locale);
    if groups.is_empty() && health.issues.is_empty() {
        empty_state(
            html,
            text(
                request.locale,
                "No degraded capability is currently reported.",
                "현재 보고된 저하 기능이 없습니다.",
            ),
        );
    } else {
        heading(
            html,
            3,
            text(
                request.locale,
                "Affected capability and scope",
                "영향받는 기능 및 범위",
            ),
        );
        html.push_str("<ul class=\"cards degradation-groups\">");
        let limit = level_limit(request.explanation_level);
        for group in groups.iter().take(limit) {
            html.push_str(&format!(
                "<li class=\"item\" data-state=\"{}\"><strong>{}</strong> · {} · {}<br><span class=\"muted\">{}: {}; {}: {}; {}: {}</span></li>",
                escape(&group.state_key),
                escape(&group.state_label),
                escape(&group.capability),
                escape(&group.language),
                escape(text(request.locale, "Affected areas", "영향 영역")),
                escape(&bounded_names(&group.areas, 3, request.locale)),
                escape(text(request.locale, "Reason", "이유")),
                escape(&bounded_names(&group.reasons, 2, request.locale)),
                escape(text(request.locale, "Usable remainder", "사용 가능한 나머지")),
                escape(&bounded_names(&group.remainders, 2, request.locale))
            ));
        }
        html.push_str("</ul>");
        rendered_bound(
            html,
            groups.len(),
            groups.len().min(limit),
            0,
            request.locale,
            "health capability groups",
        );
    }
    if let Some(step) = &projection.resume.next_meaningful_step {
        html.push_str(&format!(
            "<p class=\"next-action\"><strong>{}:</strong> {}</p>",
            escape(text(
                request.locale,
                "Grounded next action",
                "근거가 있는 다음 작업"
            )),
            escape(step)
        ));
    }
    if request.explanation_level == ExplanationLevel::Deep {
        html.push_str(&format!(
            "<details class=\"audit\"><summary>{}</summary>",
            escape(text(
                request.locale,
                "Raw diagnostic evidence",
                "원시 진단 근거"
            ))
        ));
        if health.issues.is_empty() && projection.issues.is_empty() {
            empty_state(
                html,
                text(request.locale, "No raw diagnostics.", "원시 진단 없음."),
            );
        } else {
            html.push_str("<ul class=\"audit-list\">");
            for issue in health.issues.iter().take(20) {
                list_item(
                    html,
                    &format!(
                        "{} · {} · {}",
                        health_issue_kind_label(issue.kind, request.locale),
                        issue.scope,
                        issue.detail
                    ),
                );
            }
            for issue in projection.issues.iter().take(20) {
                list_item(
                    html,
                    &format!(
                        "{} · {} · {} · {}={}",
                        projection_issue_kind_label(issue.kind, request.locale),
                        issue.affected_scope,
                        issue.reason,
                        text(request.locale, "omitted count", "생략 수"),
                        issue.omitted_count
                    ),
                );
            }
            html.push_str("</ul>");
            rendered_bound(
                html,
                health.issues.len() + projection.issues.len(),
                (health.issues.len() + projection.issues.len()).min(40),
                0,
                request.locale,
                "raw diagnostics",
            );
        }
        html.push_str("</details>");
    }
    section_end(html);
}

fn render_overview(html: &mut String, request: &ViewerRequest, projection: &ProjectProjection) {
    section_start(
        html,
        "project-overview",
        text(request.locale, "Project overview", "프로젝트 개요"),
    );
    let overview = &projection.overview;
    html.push_str(&format!(
        "<p class=\"project-identity\"><strong>{}</strong> <span class=\"muted\">{} {} · {} <code>{}</code></span></p>",
        escape(&overview.project_name),
        escape(text(request.locale, "revision", "리비전")),
        overview.canonical_revision,
        escape(text(request.locale, "Project ID", "프로젝트 ID")),
        overview.project_id
    ));
    heading(html, 3, text(request.locale, "Current goal", "현재 목표"));
    if overview.current_goals.is_empty() {
        empty_state(
            html,
            text(
                request.locale,
                "No current Project goal is recorded.",
                "현재 프로젝트 목표가 기록되지 않았습니다.",
            ),
        );
    } else {
        html.push_str("<ul class=\"goals\">");
        for goal in overview
            .current_goals
            .iter()
            .take(level_limit(request.explanation_level))
        {
            list_item(html, goal);
        }
        html.push_str("</ul>");
    }
    html.push_str(&format!(
        "<dl class=\"metrics\"><div><dt>{}</dt><dd>{}</dd></div><div><dt>{}</dt><dd>{}</dd></div><div><dt>{}</dt><dd>{}</dd></div></dl>",
        escape(text(request.locale, "Active decisions", "활성 결정")),
        overview.active_decision_count,
        escape(text(request.locale, "Open questions", "열린 질문")),
        overview.open_question_count,
        escape(text(request.locale, "Superseded decisions", "대체된 결정")),
        overview.superseded_decision_count
    ));
    heading(html, 3, text(request.locale, "Resume state", "재개 상태"));
    if let Some(checkpoint) = &projection.resume.latest_meaningful_checkpoint {
        html.push_str(&format!(
            "<p><strong>{}:</strong> {} · <strong>{}:</strong> {}</p>",
            escape(text(request.locale, "Work", "작업")),
            escape(work_state_label(checkpoint.work_state, request.locale)),
            escape(text(
                request.locale,
                "Latest checkpoint goal",
                "최근 체크포인트 목표"
            )),
            escape(&checkpoint.goal)
        ));
    } else {
        empty_state(
            html,
            text(
                request.locale,
                "No meaningful Checkpoint has been recorded yet.",
                "아직 의미 있는 체크포인트가 기록되지 않았습니다.",
            ),
        );
    }
    if let Some(step) = &projection.resume.next_meaningful_step {
        html.push_str(&format!(
            "<p class=\"next-action\"><strong>{}:</strong> {}</p>",
            escape(text(request.locale, "Next step", "다음 단계")),
            escape(step)
        ));
    } else {
        empty_state(
            html,
            text(
                request.locale,
                "No source-grounded next step is recorded.",
                "source-grounded 다음 단계가 기록되지 않았습니다.",
            ),
        );
    }
    heading(html, 3, text(request.locale, "Open Questions", "열린 질문"));
    if projection.resume.open_questions.is_empty() {
        empty_state(
            html,
            text(
                request.locale,
                "No open Questions.",
                "열린 질문이 없습니다.",
            ),
        );
    } else {
        html.push_str("<ul class=\"cards questions\">");
        for question in projection
            .resume
            .open_questions
            .iter()
            .take(level_limit(request.explanation_level))
        {
            let state = if question.on_current_frontier {
                text(request.locale, "current frontier", "현재 프런티어")
            } else {
                text(request.locale, "blocked", "차단됨")
            };
            html.push_str(&format!(
                "<li class=\"item\"><strong>{}</strong> <span class=\"badge\">{}</span><br><span class=\"muted\">{}: {}</span></li>",
                escape(&question.prompt),
                escape(state),
                escape(text(request.locale, "What the answer unlocks", "답변으로 해제되는 작업")),
                escape(&bounded_names(&question.what_the_answer_unlocks, 3, request.locale))
            ));
        }
        html.push_str("</ul>");
    }
    html.push_str(&format!(
        "<p class=\"bound\">{}: {}.</p>",
        escape(text(request.locale, "Resume omissions", "재개 요약 생략")),
        projection.resume.omitted_count
    ));
    section_end(html);
}

fn render_repository(html: &mut String, request: &ViewerRequest, projection: &ProjectProjection) {
    section_start(
        html,
        "repository-map",
        text(request.locale, "Repository Map", "저장소 지도"),
    );
    let map = &projection.repository_map;
    html.push_str(&format!(
        "<p class=\"state\" data-state=\"{}\"><strong>{}</strong> · {} {} · {} {} · {} {}</p>",
        projection_health_key(map.health),
        escape(projection_health_label(map.health, request.locale)),
        map.entities.len(),
        escape(text(request.locale, "bounded entities", "범위 제한 엔터티")),
        map.relations.len(),
        escape(text(request.locale, "bounded relations", "범위 제한 관계")),
        map.gaps.len(),
        escape(text(request.locale, "known gaps", "알려진 빈틈"))
    ));
    if map.entities.is_empty() {
        empty_state(
            html,
            text(
                request.locale,
                "No repository entities are available at the current analysis fidelity.",
                "현재 분석 정밀도에서 사용할 수 있는 저장소 엔터티가 없습니다.",
            ),
        );
    } else {
        heading(
            html,
            3,
            text(request.locale, "Structure summary", "구조 요약"),
        );
        render_repository_aggregates(html, request, projection);
        heading(
            html,
            3,
            text(
                request.locale,
                "Representative current entities",
                "대표 현재 엔터티",
            ),
        );
        let limit = level_limit(request.explanation_level);
        html.push_str("<ul class=\"cards entity-list\">");
        for entity in map.entities.iter().take(limit) {
            let locator = entity
                .source_range
                .as_ref()
                .map(|range| range.locator.as_str())
                .unwrap_or_else(|| text(request.locale, "No path recorded", "기록된 경로 없음"));
            html.push_str(&format!(
                "<li class=\"item\"><strong>{}</strong><br><span>{} · {} · {}</span><br><code>{}</code></li>",
                escape(&entity.display_name),
                escape(&language_label(&entity.language, request.locale)),
                escape(&code_entity_kind_label(&entity.kind, request.locale)),
                escape(freshness_state_label(entity.freshness.state, request.locale)),
                escape(locator)
            ));
        }
        html.push_str("</ul>");
        rendered_bound(
            html,
            map.entities.len(),
            map.entities.len().min(limit),
            projection_bound_count(projection, "repository_map.entity"),
            request.locale,
            "repository entities",
        );
    }
    heading(
        html,
        3,
        text(
            request.locale,
            "Capability coverage and gaps",
            "기능 범위 및 빈틈",
        ),
    );
    render_repository_capabilities(html, request, projection);
    if request.explanation_level == ExplanationLevel::Deep {
        html.push_str(&format!(
            "<details class=\"audit\"><summary>{}</summary>",
            escape(text(
                request.locale,
                "Opaque identities, relations, and gap evidence",
                "불투명 ID, 관계 및 빈틈 근거"
            ))
        ));
        html.push_str("<ul class=\"audit-list relations\">");
        for relation in map.relations.iter().take(20) {
            list_item(
                html,
                &format!(
                    "{} · {} · {} → {} · {} · ID {}",
                    map_relation_class_label(relation.class, request.locale),
                    relation.kind,
                    relation.source_entity,
                    relation
                        .target_entity
                        .as_deref()
                        .or(relation.unresolved_target.as_deref())
                        .unwrap_or_else(|| text(request.locale, "unresolved", "미해결")),
                    freshness_state_label(relation.freshness.state, request.locale),
                    relation.identity
                ),
            );
        }
        html.push_str("</ul>");
        rendered_bound(
            html,
            map.relations.len(),
            map.relations.len().min(20),
            projection_bound_count(projection, "repository_map.relation"),
            request.locale,
            "repository relations",
        );
        html.push_str("</details>");
    }
    section_end(html);
}

fn render_decisions(html: &mut String, request: &ViewerRequest, projection: &ProjectProjection) {
    section_start(
        html,
        "decisions",
        text(request.locale, "Current Decisions", "현재 결정"),
    );
    if projection.resume.decisions.is_empty() {
        empty_state(
            html,
            text(
                request.locale,
                "No Decisions are recorded.",
                "기록된 결정이 없습니다.",
            ),
        );
    } else {
        html.push_str("<ol class=\"cards decision-list\">");
        let limit = level_limit(request.explanation_level);
        for decision in projection.resume.decisions.iter().take(limit) {
            let link = projection
                .decision_context_code
                .iter()
                .find(|link| link.decision_id == decision.decision_id);
            html.push_str(&format!(
                "<li class=\"item\"><article><header><strong>{}</strong> <span class=\"badge\">{}</span></header>",
                escape(&decision_choice_label(&decision.choice, request.locale)),
                escape(brief_decision_state_label(decision.state, request.locale))
            ));
            html.push_str(&format!(
                "<p><strong>{}:</strong> {}</p>",
                escape(text(request.locale, "User rationale", "사용자 근거")),
                escape(decision.user_rationale.as_deref().unwrap_or_else(|| text(
                    request.locale,
                    "Not recorded",
                    "기록되지 않음"
                )))
            ));
            if let Some(link) = link {
                html.push_str(&format!(
                    "<p class=\"muted\">{}: {} · {}: {}</p>",
                    escape(text(request.locale, "Declared paths", "선언된 경로")),
                    escape(&bounded_names(&link.declared_paths, 4, request.locale)),
                    escape(text(request.locale, "Related code", "관련 코드")),
                    escape(&bounded_names(
                        &link.related_code_entities,
                        4,
                        request.locale
                    ))
                ));
                if !link.missing_or_uncertain_links.is_empty() {
                    html.push_str(&format!(
                        "<p><strong>{}:</strong> {}</p>",
                        escape(text(request.locale, "Known uncertainty", "알려진 불확실성")),
                        escape(&bounded_names(
                            &link.missing_or_uncertain_links,
                            3,
                            request.locale
                        ))
                    ));
                }
            }
            html.push_str(&format!(
                "<p class=\"record-meta\">{} {} · ID <code>{}</code></p></article></li>",
                escape(text(request.locale, "revision", "리비전")),
                decision.revision,
                decision.decision_id
            ));
        }
        html.push_str("</ol>");
        let projected = projection_bound_count(projection, "decision_context_code");
        rendered_bound(
            html,
            projection.resume.decisions.len(),
            projection.resume.decisions.len().min(limit),
            projected,
            request.locale,
            "Decisions",
        );
    }
    section_end(html);
}

fn render_checkpoints(html: &mut String, request: &ViewerRequest, projection: &ProjectProjection) {
    section_start(
        html,
        "checkpoints",
        text(request.locale, "Recent Checkpoints", "최근 체크포인트"),
    );
    if projection.checkpoint_timeline.is_empty() {
        empty_state(
            html,
            text(
                request.locale,
                "No Checkpoints have been recorded.",
                "기록된 체크포인트가 없습니다.",
            ),
        );
        section_end(html);
        return;
    }
    let limit = level_limit(request.explanation_level);
    html.push_str("<ol class=\"timeline\">");
    for entry in projection.checkpoint_timeline.iter().rev().take(limit) {
        let checkpoint = &entry.checkpoint;
        html.push_str(&format!(
            "<li class=\"item\"><article><header><strong>{}</strong> <span class=\"badge\">{} · {}</span><br><time datetime=\"unix-micros:{}\">{}</time></header>",
            escape(&checkpoint.goal),
            escape(checkpoint_kind_label(checkpoint.kind, request.locale)),
            escape(work_state_label(entry.work_state, request.locale)),
            checkpoint.recorded_at.as_unix_micros(),
            escape(&timestamp_label(checkpoint.recorded_at, request.locale))
        ));
        if let Some(change) = &checkpoint.state_change {
            html.push_str(&format!(
                "<p><strong>{}:</strong> {}</p>",
                escape(text(request.locale, "State change", "상태 변화")),
                escape(change)
            ));
        }
        html.push_str(&format!(
            "<p><strong>{}:</strong> {}</p>",
            escape(text(request.locale, "Changed paths", "변경 경로")),
            escape(&bounded_names(&checkpoint.changed_paths, 6, request.locale))
        ));
        heading(html, 4, text(request.locale, "Verification", "검증"));
        if entry.verification.is_empty() {
            empty_state(
                html,
                text(
                    request.locale,
                    "No verification recorded.",
                    "기록된 검증이 없습니다.",
                ),
            );
        } else {
            html.push_str("<ul class=\"verification-list\">");
            for fact in &entry.verification {
                let source = fact.source_id.and_then(|source_id| {
                    projection
                        .source_catalog
                        .iter()
                        .find(|basis| basis.source.id == source_id)
                });
                let label = source.map_or_else(
                    || {
                        text(request.locale, "Unlabeled verification", "레이블 없는 검증")
                            .to_owned()
                    },
                    |basis| source_display_label(&basis.source.payload, request.locale),
                );
                html.push_str(&format!(
                    "<li><strong>{}</strong>: {}",
                    escape(&label),
                    escape(verification_state_label(fact.state, request.locale))
                ));
                if let Some(outcome) = &fact.outcome {
                    html.push_str(&format!(" · {}", escape(outcome)));
                }
                if request.explanation_level == ExplanationLevel::Deep {
                    if let Some(source_id) = fact.source_id {
                        html.push_str(&format!(" · Source <code>{source_id}</code>"));
                    }
                }
                html.push_str("</li>");
            }
            html.push_str("</ul>");
        }
        html.push_str(&format!(
            "<dl class=\"fact-states\"><div><dt>{}</dt><dd>{}</dd></div><div><dt>{}</dt><dd>{}</dd></div></dl>",
            escape(text(request.locale, "User review", "사용자 검토")),
            escape(user_review_label(entry.user_review.state, request.locale)),
            escape(text(request.locale, "User acceptance", "사용자 수락")),
            escape(user_acceptance_label(entry.user_acceptance.state, request.locale))
        ));
        if !checkpoint.known_limits.is_empty() {
            html.push_str(&format!(
                "<p><strong>{}:</strong> {}</p>",
                escape(text(request.locale, "Known limits", "알려진 한계")),
                escape(&bounded_names(&checkpoint.known_limits, 4, request.locale))
            ));
        }
        html.push_str(&format!(
            "<p class=\"next-action\"><strong>{}:</strong> {}</p><p class=\"record-meta\">ID <code>{}</code> · {} {}</p></article></li>",
            escape(text(request.locale, "Next step", "다음 단계")),
            escape(&checkpoint.next_step),
            checkpoint.id,
            escape(text(request.locale, "revision", "리비전")),
            checkpoint.revision
        ));
    }
    html.push_str("</ol>");
    rendered_bound(
        html,
        projection.checkpoint_timeline.len(),
        projection.checkpoint_timeline.len().min(limit),
        projection_bound_count(projection, "checkpoint_timeline"),
        request.locale,
        "Checkpoints",
    );
    section_end(html);
}

fn render_candidates(html: &mut String, request: &ViewerRequest, projection: &ProjectProjection) {
    section_start(
        html,
        "candidates",
        text(request.locale, "Candidate inspection", "후보 검사"),
    );
    if projection.candidate_inspection.is_empty() {
        empty_state(
            html,
            text(
                request.locale,
                "No Session Candidates.",
                "세션 후보가 없습니다.",
            ),
        );
    } else {
        html.push_str("<ul class=\"cards candidate-list\">");
        let limit = level_limit(request.explanation_level);
        for candidate in projection.candidate_inspection.iter().take(limit) {
            html.push_str(&format!(
                "<li class=\"item\"><strong>{}</strong> <span class=\"badge\">{}</span><p>{}</p><p class=\"muted\">{}: {} · {}: {} · ID <code>{}</code></p></li>",
                escape(candidate_kind_label(candidate.kind, request.locale)),
                escape(inspection_health_label(candidate.health, request.locale)),
                escape(candidate.bounded_summary.as_deref().unwrap_or_else(|| text(request.locale, "Candidate content unavailable", "후보 내용을 사용할 수 없음"))),
                escape(text(request.locale, "Disposition", "처리 상태")),
                escape(&candidate_disposition_label(candidate.promotion_disposition.as_ref(), request.locale)),
                escape(text(request.locale, "Collection opt-out", "수집 제외")),
                escape(if candidate.current_applicable_opt_out.iter().any(|value| value.opted_out) { text(request.locale, "active", "활성") } else { text(request.locale, "not active", "비활성") }),
                candidate.candidate_id
            ));
        }
        html.push_str("</ul>");
        rendered_bound(
            html,
            projection.candidate_inspection.len(),
            projection.candidate_inspection.len().min(limit),
            projection_bound_count(projection, "candidate_inspection"),
            request.locale,
            "Candidates",
        );
    }
    section_end(html);
}

fn render_canonical(html: &mut String, request: &ViewerRequest, projection: &ProjectProjection) {
    section_start(
        html,
        "canonical-context",
        text(request.locale, "Canonical context", "정식 맥락"),
    );
    html.push_str(&format!(
        "<details{}><summary>{}</summary>",
        if request.explanation_level == ExplanationLevel::Deep {
            " open"
        } else {
            ""
        },
        escape(text(
            request.locale,
            "Inspect canonical records",
            "정식 기록 검사"
        ))
    ));
    if projection.canonical_inspection.is_empty() {
        empty_state(
            html,
            text(
                request.locale,
                "No canonical records.",
                "정식 기록이 없습니다.",
            ),
        );
    } else {
        let limit = level_limit(request.explanation_level);
        html.push_str("<ul class=\"canonical-list\">");
        for record in projection.canonical_inspection.iter().take(limit) {
            html.push_str(&format!(
                "<li class=\"item\"><strong>{}</strong>: {}<br><span class=\"record-meta\">{} · {} {} · ID <code>{}</code></span></li>",
                escape(canonical_kind_label(record.kind, request.locale)),
                escape(&record.summary),
                escape(canonical_lifecycle_label(&record.lifecycle_state, request.locale)),
                escape(text(request.locale, "revision", "리비전")),
                record.revision,
                escape(&record.identity)
            ));
        }
        html.push_str("</ul>");
        rendered_bound(
            html,
            projection.canonical_inspection.len(),
            projection.canonical_inspection.len().min(limit),
            projection_bound_count(projection, "canonical_inspection"),
            request.locale,
            "canonical records",
        );
    }
    html.push_str("</details>");
    section_end(html);
}

fn render_privacy(
    html: &mut String,
    request: &ViewerRequest,
    privacy: Option<&volicord_privacy::ProjectPrivacyInspection>,
) {
    section_start(
        html,
        "privacy",
        text(request.locale, "Privacy and provider", "개인정보 및 공급자"),
    );
    match privacy {
        Some(value) => {
            let configuration =
                provider_configuration_label(value.configuration_state, request.locale);
            html.push_str(&format!(
                "<p><strong>{}:</strong> {}</p>",
                escape(text(
                    request.locale,
                    "Background provider",
                    "백그라운드 공급자"
                )),
                escape(configuration)
            ));
            if let Some(event) = &value.current_opt_in {
                html.push_str(&format!(
                    "<p class=\"item\">{} · <code>{}</code> / <code>{}</code><br>{}: {} · {}: {} · {}: {}</p>",
                    escape(provider_opt_in_label(event.state, request.locale)),
                    escape(&event.policy.provider),
                    escape(&event.policy.model),
                    escape(text(request.locale, "Allowed source scope", "허용된 Source 범위")),
                    escape(&bounded_names(&event.policy.allowed_source_scopes, 5, request.locale)),
                    escape(text(request.locale, "Requests", "요청")),
                    value.requests.len(),
                    escape(text(request.locale, "Managed derived items", "관리되는 파생 항목")),
                    value.managed_derived.len()
                ));
            } else {
                empty_state(
                    html,
                    text(
                        request.locale,
                        "Local-only mode; no background provider consent",
                        "로컬 전용 모드; 백그라운드 공급자 동의 없음",
                    ),
                );
            }
        }
        None => empty_state(
            html,
            text(
                request.locale,
                "Privacy state unavailable; canonical views remain available",
                "개인정보 상태를 사용할 수 없음; 정식 맥락 보기는 계속 가능",
            ),
        ),
    }
    section_end(html);
}

fn render_documents(
    html: &mut String,
    request: &ViewerRequest,
    documents: &DocumentSet,
    request_authenticity: &str,
) {
    section_start(
        html,
        "documents",
        text(
            request.locale,
            "Document preview / export",
            "문서 미리보기 / 내보내기",
        ),
    );
    html.push_str("<div class=\"document-previews\">");
    for kind in DocumentKind::ALL {
        let document = select_document(documents, kind);
        html.push_str(&format!(
            "<details class=\"document-preview\"><summary>{}</summary><dl class=\"preview-meta\"><div><dt>{}</dt><dd>{}</dd></div><div><dt>{}</dt><dd>{}</dd></div><div><dt>{}</dt><dd>{}</dd></div><div><dt>{}</dt><dd>{}</dd></div><div><dt>{}</dt><dd>{}</dd></div></dl>",
            escape(document_kind_label(document.metadata.document_kind, request.locale)),
            escape(text(request.locale, "Language", "언어")),
            escape(&document.metadata.requested_language),
            escape(text(request.locale, "Canonical basis", "정식 근거")),
            document.metadata.canonical_revision,
            escape(text(request.locale, "Analysis snapshots", "분석 스냅샷")),
            document.metadata.analysis_snapshots.len(),
            escape(text(request.locale, "Known gaps", "알려진 빈틈")),
            document.metadata.capability_gaps.len(),
            escape(text(request.locale, "Omission reports", "생략 보고")),
            document.metadata.omissions.len()
        ));
        html.push_str(&format!(
            "<p class=\"muted\">{}: <code>{}</code> · {}: {} · {}: {} / {}</p>",
            escape(text(request.locale, "Project", "프로젝트")),
            document.metadata.project_id,
            escape(text(request.locale, "Generator", "생성기")),
            escape(&document.metadata.generator.generator),
            escape(text(
                request.locale,
                "Included Decisions / Sources",
                "포함된 결정 / Source"
            )),
            document.metadata.included_decisions.len(),
            document.metadata.used_sources.len()
        ));
        for section in &document.body.sections {
            html.push_str(&format!(
                "<section class=\"preview-section\"><h3>{}</h3>",
                escape(&section.title)
            ));
            if section.claims.is_empty() {
                empty_state(
                    html,
                    text(
                        request.locale,
                        "No grounded items.",
                        "grounded 항목이 없습니다.",
                    ),
                );
            } else {
                html.push_str("<ul class=\"preview-claims\">");
                let limit = level_limit(request.explanation_level);
                for claim in section.claims.iter().take(limit) {
                    html.push_str(&format!(
                        "<li class=\"item\"><strong>{}</strong>{} {}",
                        escape(claim_class_label(claim.class, request.locale)),
                        if claim.explicit_inference {
                            format!(
                                " <span class=\"badge\">{}</span>",
                                escape(text(request.locale, "inference", "추론"))
                            )
                        } else {
                            String::new()
                        },
                        escape(&claim.text)
                    ));
                    if request.explanation_level != ExplanationLevel::Overview {
                        html.push_str(&format!(
                            "<div class=\"record-meta\">{}: {} · {}: {} · {}: {}</div>",
                            escape(text(request.locale, "Sources", "Source")),
                            escape(&join_ids(&claim.source_basis)),
                            escape(text(request.locale, "Decisions", "결정")),
                            escape(&join_ids(&claim.decision_basis)),
                            escape(text(request.locale, "Analysis", "분석")),
                            escape(&join_ids(&claim.analysis_basis))
                        ));
                    }
                    html.push_str("</li>");
                }
                html.push_str("</ul>");
                rendered_bound(
                    html,
                    section.claims.len(),
                    section.claims.len().min(limit),
                    0,
                    request.locale,
                    "document preview claims",
                );
            }
            html.push_str("</section>");
        }
        html.push_str("</details>");
    }
    html.push_str("</div>");
    html.push_str(&format!(
        "<form class=\"action-form\" method=\"post\" action=\"/documents/export\"><fieldset><legend>{}</legend><label>{} <select name=\"kind\">",
        escape(text(request.locale, "Export generated document", "생성 문서 내보내기")),
        escape(text(request.locale, "Document", "문서"))
    ));
    for kind in DocumentKind::ALL {
        html.push_str(&format!(
            "<option value=\"{}\">{}</option>",
            escape(kind.slug()),
            escape(document_kind_label(kind, request.locale))
        ));
    }
    html.push_str(&format!("</select></label> <label>{} <select name=\"format\"><option value=\"markdown\">Markdown</option><option value=\"html\">HTML</option></select></label> <label>{} <input name=\"destination\" required></label>", escape(text(request.locale, "Format", "형식")), escape(text(request.locale, "Absolute destination", "절대 대상 경로"))));
    render_view_fields(html, request, request_authenticity);
    html.push_str(&format!(
        "<button type=\"submit\">{}</button></fieldset></form>",
        escape(text(request.locale, "Export", "내보내기"))
    ));
    empty_state(html, text(request.locale, "Export writes only to an explicit absolute destination and never adopts the document automatically.", "내보내기는 명시한 절대 경로에만 쓰며 문서를 자동 채택하지 않습니다."));
    section_end(html);
}

fn render_mutation_controls(
    html: &mut String,
    request: &ViewerRequest,
    projection: &ProjectProjection,
    request_authenticity: &str,
) {
    section_start(
        html,
        "memory-actions",
        text(request.locale, "Memory actions", "기억 작업"),
    );
    empty_state(html, text(request.locale, "Correction, supersession, and forgetting are submitted to Local Operations with explicit current-host user input. The Viewer does not own canonical mutation authority.", "수정, 대체 및 삭제는 명시적인 현재 호스트 사용자 입력과 함께 로컬 작업 계층에 제출됩니다. 뷰어는 정식 변경 권한을 소유하지 않습니다."));
    let mut action_count = 0_usize;
    for record in &projection.canonical_inspection {
        match record.kind {
            CanonicalInspectionKind::ContextItem => {
                action_count += 1;
                html.push_str(&format!("<details class=\"memory-target\"><summary><strong>{}</strong>: {} <span class=\"record-meta\">({}; {} {}; ID <code>{}</code>)</span></summary><form class=\"action-form\" method=\"post\" action=\"/memory/context/correct\"><fieldset><legend>{}</legend>", escape(text(request.locale, "Context Item", "맥락 항목")), escape(&record.summary), escape(canonical_lifecycle_label(&record.lifecycle_state, request.locale)), escape(text(request.locale, "revision", "리비전")), record.revision, escape(&record.identity), escape(text(request.locale, "Correct this current value", "이 현재 값 수정"))));
                hidden(html, "record_id", &record.identity);
                hidden(html, "expected_revision", &record.revision.to_string());
                html.push_str(&format!("<label>{} <textarea name=\"corrected_text\" required></textarea></label><label>{} <textarea name=\"user_turn\" required></textarea></label>", escape(text(request.locale, "Corrected statement", "수정한 진술")), escape(text(request.locale, "Current user turn", "현재 사용자 입력"))));
                render_view_fields(html, request, request_authenticity);
                html.push_str(&format!(
                    "<button type=\"submit\">{}</button></fieldset></form></details>",
                    escape(text(request.locale, "Correct", "수정"))
                ));
            }
            CanonicalInspectionKind::Decision => {
                action_count += 1;
                html.push_str(&format!("<details class=\"memory-target\"><summary><strong>{}</strong>: {} <span class=\"record-meta\">({}; {} {}; ID <code>{}</code>)</span></summary><form class=\"action-form\" method=\"post\" action=\"/memory/decision/correct\"><fieldset><legend>{}</legend>", escape(text(request.locale, "Decision", "결정")), escape(&record.summary), escape(canonical_lifecycle_label(&record.lifecycle_state, request.locale)), escape(text(request.locale, "revision", "리비전")), record.revision, escape(&record.identity), escape(text(request.locale, "Correct this rationale", "이 근거 수정"))));
                hidden(html, "record_id", &record.identity);
                hidden(html, "expected_revision", &record.revision.to_string());
                html.push_str(&format!("<label>{} <textarea name=\"corrected_text\" required></textarea></label><label>{} <textarea name=\"user_turn\" required></textarea></label>", escape(text(request.locale, "Corrected rationale", "수정한 근거")), escape(text(request.locale, "Current user turn", "현재 사용자 입력"))));
                render_view_fields(html, request, request_authenticity);
                html.push_str(&format!("<button type=\"submit\">{}</button></fieldset></form><form class=\"action-form\" method=\"post\" action=\"/memory/decision/supersede\"><fieldset><legend>{}</legend>", escape(text(request.locale, "Correct rationale", "근거 수정")), escape(text(request.locale, "Supersede this Decision", "이 결정 대체"))));
                hidden(html, "record_id", &record.identity);
                html.push_str(&format!("<label>{} <input name=\"alternative\" required></label><label>{} <textarea name=\"rationale\"></textarea></label><label>{} <textarea name=\"user_turn\" required></textarea></label>", escape(text(request.locale, "New displayed alternative key", "새 표시 대안 키")), escape(text(request.locale, "Rationale", "근거")), escape(text(request.locale, "Current user turn", "현재 사용자 입력"))));
                render_view_fields(html, request, request_authenticity);
                html.push_str(&format!(
                    "<button type=\"submit\">{}</button></fieldset></form></details>",
                    escape(text(request.locale, "Supersede", "대체"))
                ));
            }
            _ => {}
        }
        if let Some(kind) = forgettable_kind(record.kind) {
            action_count += 1;
            html.push_str(&format!("<details class=\"memory-target destructive\"><summary><strong>{} {}</strong>: {} <span class=\"record-meta\">({}; {} {}; ID <code>{}</code>)</span></summary><form class=\"action-form\" method=\"post\" action=\"/memory/forget\"><fieldset><legend>{}</legend>", escape(text(request.locale, "Forget", "삭제")), escape(canonical_kind_label(record.kind, request.locale)), escape(&record.summary), escape(canonical_lifecycle_label(&record.lifecycle_state, request.locale)), escape(text(request.locale, "revision", "리비전")), record.revision, escape(&record.identity), escape(text(request.locale, "Confirm the identifiable current target", "식별 가능한 현재 대상 확인"))));
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
                "<button type=\"submit\">{}</button></fieldset></form></details>",
                escape(text(request.locale, "Forget this record", "이 기록 삭제"))
            ));
        }
    }
    if action_count == 0 {
        empty_state(
            html,
            text(
                request.locale,
                "No correctable or forgettable records are in the bounded inspection.",
                "범위 제한 검사에 수정하거나 삭제할 수 있는 기록이 없습니다.",
            ),
        );
    }
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
    html.push_str(&format!("<div class=\"button-row\"><button name=\"decision\" value=\"confirm\" type=\"submit\">{}</button> <button name=\"decision\" value=\"deny\" type=\"submit\">{}</button></div></fieldset></form>", escape(text(request.locale, "Confirm exact effect", "정확한 효과 확인")), escape(text(request.locale, "Deny", "거부"))));
    section_end(html);
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DegradationGroup {
    state_key: String,
    state_label: String,
    capability: String,
    language: String,
    areas: Vec<String>,
    reasons: Vec<String>,
    remainders: Vec<String>,
}

fn degradation_groups(
    projection: &ProjectProjection,
    locale: ViewerLocale,
) -> Vec<DegradationGroup> {
    let mut groups = BTreeMap::<(String, String, String), DegradationGroup>::new();
    for gap in &projection.repository_map.gaps {
        let state_key = capability_state_key(gap.state).to_owned();
        let capability = capability_label(gap.capability, locale).to_owned();
        let language = gap.language.as_ref().map_or_else(
            || text(locale, "all languages", "모든 언어").to_owned(),
            |language| language_label(language, locale),
        );
        let group = groups
            .entry((state_key.clone(), capability.clone(), language.clone()))
            .or_insert_with(|| DegradationGroup {
                state_key,
                state_label: capability_state_label(gap.state, locale).to_owned(),
                capability,
                language,
                areas: Vec::new(),
                reasons: Vec::new(),
                remainders: Vec::new(),
            });
        group.areas.push(gap.area.clone());
        group.reasons.push(gap.reason.clone());
        if let Some(remainder) = &gap.usable_remainder {
            group.remainders.push(remainder.clone());
        }
    }
    groups.into_values().collect()
}

fn render_repository_aggregates(
    html: &mut String,
    request: &ViewerRequest,
    projection: &ProjectProjection,
) {
    let mut languages = BTreeMap::<String, usize>::new();
    let mut kinds = BTreeMap::<String, usize>::new();
    for entity in &projection.repository_map.entities {
        *languages
            .entry(language_label(&entity.language, request.locale))
            .or_default() += 1;
        *kinds
            .entry(code_entity_kind_label(&entity.kind, request.locale))
            .or_default() += 1;
    }
    html.push_str("<div class=\"aggregate-grid\">");
    aggregate_card(
        html,
        text(request.locale, "By language", "언어별"),
        &languages,
        request.locale,
    );
    aggregate_card(
        html,
        text(request.locale, "By entity kind", "엔터티 종류별"),
        &kinds,
        request.locale,
    );
    html.push_str("</div>");
    let omitted = projection_bound_count(projection, "repository_map.entity");
    if omitted > 0 {
        html.push_str(&format!(
            "<p class=\"bound\">{} {}. {}</p>",
            omitted,
            escape(text(
                request.locale,
                "entities are outside this deterministic projection bound",
                "개 엔터티가 이 결정적 프로젝션 범위 밖에 있습니다"
            )),
            escape(text(
                request.locale,
                "Aggregate counts describe the bounded projection shown here.",
                "집계 수는 여기 표시된 범위 제한 프로젝션을 설명합니다."
            ))
        ));
    }
}

fn aggregate_card(
    html: &mut String,
    title: &str,
    values: &BTreeMap<String, usize>,
    locale: ViewerLocale,
) {
    html.push_str(&format!(
        "<article class=\"aggregate-card\"><h4>{}</h4>",
        escape(title)
    ));
    if values.is_empty() {
        empty_state(html, text(locale, "No data.", "데이터 없음."));
    } else {
        html.push_str("<dl>");
        for (label, count) in values {
            html.push_str(&format!(
                "<div><dt>{}</dt><dd>{}</dd></div>",
                escape(label),
                count
            ));
        }
        html.push_str("</dl>");
    }
    html.push_str("</article>");
}

fn render_repository_capabilities(
    html: &mut String,
    request: &ViewerRequest,
    projection: &ProjectProjection,
) {
    let reports = &projection.repository_map.capabilities;
    if reports.is_empty() {
        empty_state(
            html,
            text(
                request.locale,
                "No capability report is available.",
                "사용 가능한 기능 보고가 없습니다.",
            ),
        );
        return;
    }
    let limit = level_limit(request.explanation_level);
    html.push_str("<ul class=\"cards capability-list\">");
    for report in reports.iter().take(limit) {
        let language = report.language.as_ref().map_or_else(
            || text(request.locale, "all languages", "모든 언어").to_owned(),
            |language| language_label(language, request.locale),
        );
        html.push_str(&format!(
            "<li class=\"item\" data-state=\"{}\"><strong>{}</strong> · {} · <code>{}</code><br><span>{}: {} · files {} · entities {} · relations {}</span>",
            capability_state_key(report.state),
            escape(capability_label(report.capability, request.locale)),
            escape(&language),
            escape(&report.area.path),
            escape(text(request.locale, "State", "상태")),
            escape(capability_state_label(report.state, request.locale)),
            report.coverage.covered_file_count,
            report.coverage.covered_entity_count,
            report.coverage.covered_relation_count
        ));
        if let Some(reason) = &report.reason {
            html.push_str(&format!(
                "<br><span class=\"muted\">{}: {}</span>",
                escape(text(request.locale, "Reason", "이유")),
                escape(reason)
            ));
        }
        if let Some(remainder) = &report.usable_remainder {
            html.push_str(&format!(
                "<br><span class=\"muted\">{}: {}</span>",
                escape(text(
                    request.locale,
                    "Usable remainder",
                    "사용 가능한 나머지"
                )),
                escape(remainder)
            ));
        }
        html.push_str("</li>");
    }
    html.push_str("</ul>");
    rendered_bound(
        html,
        reports.len(),
        reports.len().min(limit),
        projection_bound_count(projection, "repository_map.capability"),
        request.locale,
        "capability reports",
    );
    let groups = degradation_groups(projection, request.locale);
    if groups.is_empty() {
        empty_state(
            html,
            text(
                request.locale,
                "No known capability gaps.",
                "알려진 기능 빈틈이 없습니다.",
            ),
        );
    } else {
        html.push_str(&format!(
            "<details><summary>{}</summary><ul class=\"gap-list\">",
            escape(text(request.locale, "Known gap detail", "알려진 빈틈 상세"))
        ));
        for group in groups.iter().take(limit) {
            html.push_str(&format!(
                "<li class=\"item\" data-state=\"{}\"><strong>{}</strong> · {} · {}<br>{}: {}<br>{}: {}<br>{}: {}</li>",
                escape(&group.state_key),
                escape(&group.state_label),
                escape(&group.capability),
                escape(&group.language),
                escape(text(request.locale, "Areas", "영역")),
                escape(&bounded_names(&group.areas, 4, request.locale)),
                escape(text(request.locale, "Reason", "이유")),
                escape(&bounded_names(&group.reasons, 3, request.locale)),
                escape(text(request.locale, "Usable remainder", "사용 가능한 나머지")),
                escape(&bounded_names(&group.remainders, 3, request.locale))
            ));
        }
        html.push_str("</ul>");
        rendered_bound(
            html,
            groups.len(),
            groups.len().min(limit),
            projection_bound_count(projection, "repository_map.gap"),
            request.locale,
            "gap groups",
        );
        html.push_str("</details>");
    }
}

fn status_item(html: &mut String, request: &ViewerRequest, label: &str, available: bool) {
    html.push_str(&format!(
        "<li><strong>{}:</strong> {}</li>",
        escape(label),
        escape(if available {
            text(request.locale, "available", "사용 가능")
        } else {
            text(request.locale, "unavailable", "사용 불가")
        })
    ));
}

fn level_limit(level: ExplanationLevel) -> usize {
    match level {
        ExplanationLevel::Overview => 4,
        ExplanationLevel::Working => 8,
        ExplanationLevel::Deep => 12,
    }
}

fn projection_bound_count(projection: &ProjectProjection, scope: &str) -> usize {
    projection
        .issues
        .iter()
        .filter(|issue| issue.kind == ProjectionIssueKind::Bound && issue.affected_scope == scope)
        .map(|issue| issue.omitted_count)
        .sum()
}

fn rendered_bound(
    html: &mut String,
    bounded_count: usize,
    displayed_count: usize,
    projection_omitted: usize,
    locale: ViewerLocale,
    scope: &str,
) {
    let omitted = bounded_count
        .saturating_sub(displayed_count)
        .saturating_add(projection_omitted);
    html.push_str(&format!(
        "<p class=\"bound\" data-bound-scope=\"{}\">{} {} · {} {}.</p>",
        escape(scope),
        displayed_count,
        escape(text(locale, "shown", "표시")),
        omitted,
        escape(text(
            locale,
            "omitted by deterministic bounds",
            "결정적 범위 제한으로 생략"
        ))
    ));
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

const fn explanation_level_label(level: ExplanationLevel, locale: ViewerLocale) -> &'static str {
    match level {
        ExplanationLevel::Overview => text(locale, "Overview", "개요"),
        ExplanationLevel::Working => text(locale, "Working", "작업"),
        ExplanationLevel::Deep => text(locale, "Deep / audit", "심층 / 감사"),
    }
}

const fn health_state_key(state: HealthState) -> &'static str {
    match state {
        HealthState::Healthy => "healthy",
        HealthState::Degraded => "degraded",
        HealthState::Failed => "failed",
    }
}

const fn health_state_label(state: HealthState, locale: ViewerLocale) -> &'static str {
    match state {
        HealthState::Healthy => text(locale, "healthy", "정상"),
        HealthState::Degraded => text(locale, "degraded", "저하됨"),
        HealthState::Failed => text(locale, "failed", "실패"),
    }
}

const fn health_issue_kind_label(kind: HealthIssueKind, locale: ViewerLocale) -> &'static str {
    match kind {
        HealthIssueKind::Unavailable => text(locale, "unavailable", "사용 불가"),
        HealthIssueKind::Unsupported => text(locale, "unsupported", "지원하지 않음"),
        HealthIssueKind::Failed => text(locale, "failed", "실패"),
        HealthIssueKind::Stale => text(locale, "stale", "오래됨"),
        HealthIssueKind::Corrupt => text(locale, "corrupt", "손상됨"),
        HealthIssueKind::RepairRequired => text(locale, "repair required", "복구 필요"),
    }
}

const fn projection_health_key(state: ProjectionHealth) -> &'static str {
    match state {
        ProjectionHealth::Complete => "complete",
        ProjectionHealth::Partial => "partial",
        ProjectionHealth::Degraded => "degraded",
    }
}

const fn projection_health_label(state: ProjectionHealth, locale: ViewerLocale) -> &'static str {
    match state {
        ProjectionHealth::Complete => text(locale, "complete", "완전"),
        ProjectionHealth::Partial => text(locale, "partial", "일부"),
        ProjectionHealth::Degraded => text(locale, "degraded", "저하됨"),
    }
}

const fn projection_issue_kind_label(
    kind: ProjectionIssueKind,
    locale: ViewerLocale,
) -> &'static str {
    match kind {
        ProjectionIssueKind::Bound => text(locale, "bounded omission", "범위 제한 생략"),
        ProjectionIssueKind::WrongProject => text(locale, "wrong Project", "다른 프로젝트"),
        ProjectionIssueKind::PartialCapability => text(locale, "partial capability", "일부 기능"),
        ProjectionIssueKind::UnavailableCapability => {
            text(locale, "unavailable capability", "사용 불가 기능")
        }
        ProjectionIssueKind::UnsupportedCapability => {
            text(locale, "unsupported capability", "미지원 기능")
        }
        ProjectionIssueKind::FailedCapability => text(locale, "failed capability", "실패한 기능"),
        ProjectionIssueKind::StaleCapability => text(locale, "stale capability", "오래된 기능"),
        ProjectionIssueKind::SourceUnavailable => {
            text(locale, "Source unavailable", "Source 사용 불가")
        }
        ProjectionIssueKind::SourceStale => text(locale, "Source stale", "Source 오래됨"),
        ProjectionIssueKind::CandidateInspection => {
            text(locale, "Candidate inspection", "후보 검사")
        }
    }
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

const fn capability_state_key(state: CapabilityState) -> &'static str {
    match state {
        CapabilityState::Available => "available",
        CapabilityState::Unavailable => "unavailable",
        CapabilityState::Unsupported => "unsupported",
        CapabilityState::Partial => "partial",
        CapabilityState::Failed => "failed",
        CapabilityState::Stale => "stale",
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

fn language_label(language: &Language, locale: ViewerLocale) -> String {
    match language {
        Language::Java => "Java".to_owned(),
        Language::Python => "Python".to_owned(),
        Language::JavaScript => "JavaScript".to_owned(),
        Language::TypeScript => "TypeScript".to_owned(),
        Language::C => "C".to_owned(),
        Language::Cpp => "C++".to_owned(),
        Language::Rust => "Rust".to_owned(),
        Language::Markdown => "Markdown".to_owned(),
        Language::Json => "JSON".to_owned(),
        Language::Yaml => "YAML".to_owned(),
        Language::Toml => "TOML".to_owned(),
        Language::Xml => "XML".to_owned(),
        Language::Shell => text(locale, "Shell", "셸").to_owned(),
        Language::Go => "Go".to_owned(),
        Language::OtherText(value) => value.clone(),
        Language::UnknownText => text(locale, "unknown text", "알 수 없는 텍스트").to_owned(),
    }
}

fn code_entity_kind_label(kind: &CodeEntityKind, locale: ViewerLocale) -> String {
    match kind {
        CodeEntityKind::Repository => text(locale, "repository", "저장소").to_owned(),
        CodeEntityKind::Package => text(locale, "package", "패키지").to_owned(),
        CodeEntityKind::Module => text(locale, "module", "모듈").to_owned(),
        CodeEntityKind::Namespace => text(locale, "namespace", "네임스페이스").to_owned(),
        CodeEntityKind::File => text(locale, "file", "파일").to_owned(),
        CodeEntityKind::Class => text(locale, "class", "클래스").to_owned(),
        CodeEntityKind::Interface => text(locale, "interface", "인터페이스").to_owned(),
        CodeEntityKind::Trait => "trait".to_owned(),
        CodeEntityKind::Struct => "struct".to_owned(),
        CodeEntityKind::Enum => "enum".to_owned(),
        CodeEntityKind::Type => text(locale, "type", "타입").to_owned(),
        CodeEntityKind::Function => text(locale, "function", "함수").to_owned(),
        CodeEntityKind::Method => text(locale, "method", "메서드").to_owned(),
        CodeEntityKind::Field => text(locale, "field", "필드").to_owned(),
        CodeEntityKind::Test => text(locale, "test", "테스트").to_owned(),
        CodeEntityKind::Configuration => text(locale, "configuration", "설정").to_owned(),
        CodeEntityKind::Document => text(locale, "document", "문서").to_owned(),
        CodeEntityKind::LanguageSpecific(value) => value.clone(),
    }
}

const fn freshness_state_label(state: FreshnessState, locale: ViewerLocale) -> &'static str {
    match state {
        FreshnessState::Current => text(locale, "current", "현재"),
        FreshnessState::Stale => text(locale, "stale", "오래됨"),
        FreshnessState::Unknown => text(locale, "unknown", "알 수 없음"),
    }
}

const fn map_relation_class_label(class: MapRelationClass, locale: ViewerLocale) -> &'static str {
    match class {
        MapRelationClass::StructuralFact => text(locale, "structural fact", "구조 사실"),
        MapRelationClass::SemanticResult => text(locale, "semantic result", "의미 분석 결과"),
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

const fn checkpoint_kind_label(kind: CheckpointKind, locale: ViewerLocale) -> &'static str {
    match kind {
        CheckpointKind::Completion => text(locale, "completion", "완료"),
        CheckpointKind::Pause => text(locale, "pause", "일시 중지"),
        CheckpointKind::Handoff => text(locale, "handoff", "인계"),
    }
}

const fn work_state_label(state: WorkState, locale: ViewerLocale) -> &'static str {
    match state {
        WorkState::InProgress => text(locale, "in progress", "진행 중"),
        WorkState::Paused => text(locale, "paused", "일시 중지"),
        WorkState::Completed => text(locale, "completed", "완료"),
        WorkState::Abandoned => text(locale, "abandoned", "중단"),
        WorkState::Superseded => text(locale, "superseded", "대체됨"),
    }
}

const fn verification_state_label(state: VerificationState, locale: ViewerLocale) -> &'static str {
    match state {
        VerificationState::NotRun => text(locale, "not run", "실행하지 않음"),
        VerificationState::Partial => text(locale, "partial", "일부 검증"),
        VerificationState::Passed => text(locale, "passed", "통과"),
        VerificationState::Failed => text(locale, "failed", "실패"),
    }
}

const fn user_review_label(state: UserReviewState, locale: ViewerLocale) -> &'static str {
    match state {
        UserReviewState::NotRequested => text(locale, "not requested", "요청하지 않음"),
        UserReviewState::Pending => text(locale, "pending", "대기 중"),
        UserReviewState::Reviewed => text(locale, "reviewed", "검토됨"),
    }
}

const fn user_acceptance_label(state: UserAcceptanceState, locale: ViewerLocale) -> &'static str {
    match state {
        UserAcceptanceState::NotRequested => text(locale, "not requested", "요청하지 않음"),
        UserAcceptanceState::Pending => text(locale, "pending", "대기 중"),
        UserAcceptanceState::Accepted => text(locale, "accepted", "수락됨"),
        UserAcceptanceState::Rejected => text(locale, "rejected", "거부됨"),
    }
}

fn source_display_label(payload: &SourcePayload, locale: ViewerLocale) -> String {
    match payload {
        SourcePayload::RepositorySnapshot { revision } => format!(
            "{} {}",
            text(locale, "Repository snapshot", "저장소 스냅샷"),
            revision
        ),
        SourcePayload::RepositoryCommit { commit } => format!(
            "{} {}",
            text(locale, "Repository commit", "저장소 커밋"),
            commit
        ),
        SourcePayload::File { locator, .. } => locator.clone(),
        SourcePayload::Symbol { locator, .. } => locator.clone(),
        SourcePayload::CommandExecution { command_label, .. } => command_label.clone(),
        SourcePayload::CurrentHostUserTurn { host, .. } => format!(
            "{} ({host})",
            text(locale, "Current-host user input", "현재 호스트 사용자 입력")
        ),
        SourcePayload::Url { url } => url.clone(),
        SourcePayload::AdoptedArtifact { locator, .. } => locator.clone(),
    }
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

const fn candidate_kind_label(kind: Option<CandidateKind>, locale: ViewerLocale) -> &'static str {
    match kind {
        Some(CandidateKind::Observation) => text(locale, "observation", "관찰"),
        Some(CandidateKind::Hypothesis) => text(locale, "hypothesis", "가설"),
        Some(CandidateKind::SemanticClaim) => text(locale, "semantic claim", "의미 주장"),
        Some(CandidateKind::QuestionCandidate) => text(locale, "Question candidate", "질문 후보"),
        Some(CandidateKind::CheckpointCandidate) => {
            text(locale, "Checkpoint candidate", "체크포인트 후보")
        }
        Some(CandidateKind::PromotionProposal) => text(locale, "promotion proposal", "승격 제안"),
        None => text(locale, "unknown Candidate kind", "알 수 없는 후보 종류"),
    }
}

const fn inspection_health_label(health: InspectionHealth, locale: ViewerLocale) -> &'static str {
    match health {
        InspectionHealth::Complete => text(locale, "complete", "완전"),
        InspectionHealth::Partial => text(locale, "partial", "일부"),
        InspectionHealth::Degraded => text(locale, "degraded", "저하됨"),
        InspectionHealth::NotFound => text(locale, "not found", "찾을 수 없음"),
    }
}

fn candidate_disposition_label(
    disposition: Option<&CandidateDisposition>,
    locale: ViewerLocale,
) -> String {
    match disposition {
        Some(CandidateDisposition::PendingOrRetained) => {
            text(locale, "pending or retained", "대기 또는 보존").to_owned()
        }
        Some(CandidateDisposition::Promoted {
            canonical_question_id,
            ..
        }) => format!(
            "{}: {}",
            text(locale, "promoted to Question", "질문으로 승격"),
            canonical_question_id
        ),
        Some(CandidateDisposition::Dismissed { reason, .. }) => {
            format!("{}: {}", text(locale, "dismissed", "기각됨"), reason)
        }
        Some(CandidateDisposition::ExpiredOrRetentionCleaned) => text(
            locale,
            "expired or retention-cleaned",
            "만료 또는 보존 정리됨",
        )
        .to_owned(),
        None => text(locale, "not reported", "보고되지 않음").to_owned(),
    }
}

const fn canonical_kind_label(kind: CanonicalInspectionKind, locale: ViewerLocale) -> &'static str {
    match kind {
        CanonicalInspectionKind::Project => text(locale, "Project", "프로젝트"),
        CanonicalInspectionKind::Source => "Source",
        CanonicalInspectionKind::Question => text(locale, "Question", "질문"),
        CanonicalInspectionKind::Decision => text(locale, "Decision", "결정"),
        CanonicalInspectionKind::ContextItem => text(locale, "Context Item", "맥락 항목"),
        CanonicalInspectionKind::Checkpoint => text(locale, "Checkpoint", "체크포인트"),
    }
}

fn canonical_lifecycle_label(value: &str, locale: ViewerLocale) -> &str {
    match value {
        "current" | "Current" => text(locale, "current", "현재"),
        "active" => text(locale, "active", "활성"),
        "superseded" | "Superseded" => text(locale, "superseded", "대체됨"),
        "review_due" => text(locale, "review due", "검토 필요"),
        "open" | "Open" => text(locale, "open", "열림"),
        "terminal" | "Terminal" => text(locale, "terminal", "종료"),
        "in_progress" | "InProgress" => text(locale, "in progress", "진행 중"),
        "paused" | "Paused" => text(locale, "paused", "일시 중지"),
        "completed" | "Completed" => text(locale, "completed", "완료"),
        "abandoned" | "Abandoned" => text(locale, "abandoned", "중단"),
        "stale" | "Stale" => text(locale, "stale", "오래됨"),
        "unavailable" | "Unavailable" => text(locale, "unavailable", "사용 불가"),
        "unknown" | "Unknown" => text(locale, "unknown", "알 수 없음"),
        _ => value,
    }
}

const fn provider_configuration_label(
    state: ProviderConfigurationState,
    locale: ViewerLocale,
) -> &'static str {
    match state {
        ProviderConfigurationState::NeverEnabled => {
            text(locale, "never enabled", "활성화한 적 없음")
        }
        ProviderConfigurationState::Enabled => text(locale, "enabled", "활성화됨"),
        ProviderConfigurationState::Disabled => text(locale, "disabled", "비활성화됨"),
        ProviderConfigurationState::Revoked => text(locale, "revoked", "철회됨"),
    }
}

const fn provider_opt_in_label(state: ProviderOptInState, locale: ViewerLocale) -> &'static str {
    match state {
        ProviderOptInState::Enabled => text(locale, "enabled", "활성화됨"),
        ProviderOptInState::Disabled => text(locale, "disabled", "비활성화됨"),
        ProviderOptInState::Revoked => text(locale, "revoked", "철회됨"),
    }
}

const fn document_kind_label(kind: DocumentKind, locale: ViewerLocale) -> &'static str {
    match kind {
        DocumentKind::ProjectArchitectureGuide => text(
            locale,
            "Project & Architecture Guide",
            "프로젝트 및 아키텍처 가이드",
        ),
        DocumentKind::DecisionReport => text(locale, "Decision Report", "결정 보고서"),
        DocumentKind::ImplementationPlan => text(locale, "Implementation Plan", "구현 계획"),
        DocumentKind::HandoffResume => text(locale, "Handoff / Resume", "인계 / 재개"),
    }
}

const fn claim_class_label(class: ClaimClass, locale: ViewerLocale) -> &'static str {
    match class {
        ClaimClass::CanonicalContext => text(locale, "Canonical Context", "정식 맥락"),
        ClaimClass::RepositoryObservation => text(locale, "Repository Observation", "저장소 관찰"),
        ClaimClass::StructuralFact => text(locale, "Structural Fact", "구조 사실"),
        ClaimClass::SemanticResult => text(locale, "Semantic Result", "의미 분석 결과"),
        ClaimClass::AgentInterpretation => text(locale, "Agent Interpretation", "에이전트 해석"),
    }
}

fn join_ids<T: fmt::Display>(values: &[T]) -> String {
    if values.is_empty() {
        "—".to_owned()
    } else {
        values
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    }
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

fn section_start(html: &mut String, id: &str, title: &str) {
    html.push_str(&format!(
        "<section id=\"{}\" aria-labelledby=\"{}-heading\"><h2 id=\"{}-heading\">{}</h2>",
        escape(id),
        escape(id),
        escape(id),
        escape(title)
    ));
}

fn section_end(html: &mut String) {
    html.push_str("</section>");
}

fn list_item(html: &mut String, value: &str) {
    html.push_str(&format!("<li class=\"item\">{}</li>", escape(value)));
}

fn empty_state(html: &mut String, value: &str) {
    html.push_str(&format!("<p class=\"empty-state\">{}</p>", escape(value)));
}

const fn text<'a>(locale: ViewerLocale, english: &'a str, korean: &'a str) -> &'a str {
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
:root{color-scheme:light dark;font-family:system-ui,sans-serif;line-height:1.55}*{box-sizing:border-box}body{margin:0;background:#111827;color:#e5e7eb}main{max-width:72rem;margin:auto;padding:clamp(1rem,4vw,2.5rem)}h1,h2,h3,h4{color:#f9fafb;overflow-wrap:anywhere}h2{border-top:1px solid #374151;padding-top:1.25rem}a{color:#93c5fd;text-underline-offset:.2em}a:focus-visible,button:focus-visible,input:focus-visible,textarea:focus-visible,select:focus-visible,summary:focus-visible{outline:.22rem solid #fbbf24;outline-offset:.18rem}.level-nav{display:flex;flex-wrap:wrap;gap:.5rem;list-style:none;padding:0}.level-nav a{display:block;padding:.45rem .7rem;border:1px solid #4b5563;border-radius:.4rem}.level-nav a[aria-current=page]{background:#dbeafe;color:#111827;font-weight:700}.item,details,.state,.guarded,.aggregate-card{padding:.7rem .85rem;margin:.5rem 0;background:#1f2937;border-radius:.45rem;border:1px solid #374151}.state[data-state=degraded],.item[data-state=partial],.item[data-state=unsupported],.item[data-state=stale]{border-left:.35rem solid #f59e0b}.state[data-state=failed],.item[data-state=failed],.item[data-state=unavailable]{border-left:.35rem solid #ef4444}.state[data-state=healthy],.state[data-state=complete],.item[data-state=available]{border-left:.35rem solid #22c55e}.badge{display:inline-block;padding:.05rem .4rem;border:1px solid #6b7280;border-radius:999px;font-size:.9em}.guarded{border:2px solid #f59e0b}.muted,.record-meta,.bound{color:#cbd5e1;font-size:.92rem}.empty-state{padding:.65rem .8rem;border:1px dashed #6b7280;border-radius:.45rem;color:#d1d5db}.next-action{padding:.75rem;border-left:.35rem solid #60a5fa;background:#172554}.cards,.timeline,.canonical-list,.preview-claims,.verification-list,.audit-list,.status-summary,.goals,.gap-list{padding-left:1.35rem}.metrics,.fact-states,.preview-meta{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(12rem,100%),1fr));gap:.5rem}.metrics div,.fact-states div,.preview-meta div,.aggregate-card dl div{padding:.4rem}.metrics dt,.fact-states dt,.preview-meta dt,.aggregate-card dt{font-weight:700}.metrics dd,.fact-states dd,.preview-meta dd,.aggregate-card dd{margin:0}.aggregate-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(18rem,100%),1fr));gap:.75rem}.document-previews{display:grid;gap:.65rem}.preview-section{padding-left:.65rem;border-left:1px solid #4b5563}code{white-space:pre-wrap;overflow-wrap:anywhere}.action-form{display:grid;gap:.65rem;margin:.75rem 0}.action-form fieldset{display:grid;gap:.6rem;min-width:0;border:1px solid #4b5563;border-radius:.45rem}.action-form legend{font-weight:700}.action-form label{display:grid;gap:.25rem;min-width:0}textarea,input,select,button{font:inherit;padding:.5rem;max-width:100%}textarea{min-height:5rem;resize:vertical}button{width:max-content;min-height:2.75rem}.button-row{display:flex;flex-wrap:wrap;gap:.5rem}.destructive{border-color:#ef4444}summary{cursor:pointer;overflow-wrap:anywhere}
@media (max-width:44rem){main{padding:1rem}.level-nav{display:grid;grid-template-columns:1fr}.level-nav a{width:100%}.metrics,.fact-states,.preview-meta,.aggregate-grid{grid-template-columns:1fr}.item,details,.state,.guarded,.aggregate-card{padding:.65rem}.cards,.timeline,.canonical-list,.preview-claims,.verification-list,.audit-list,.status-summary,.goals,.gap-list{padding-left:1.05rem}.button-row button,button{width:100%}}
</style>"#;

#[allow(dead_code)]
fn _projection_health_is_explicit(value: ProjectionHealth) -> &'static str {
    match value {
        ProjectionHealth::Complete => "complete",
        ProjectionHealth::Partial => "partial",
        ProjectionHealth::Degraded => "degraded",
    }
}
