use crate::{
    BriefDecisionState, CapabilityGap, MapRelationClass, ProjectProjection, ProjectionIssue,
};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error as StdError;
use std::fmt;
use volicord_context::{
    ContextItemRole, DecisionChoice, DecisionId, ProjectId, SourceFreshness, SourceId,
    TimestampMicros, UserAcceptanceState, UserReviewState, VerificationState, WorkState,
};
use volicord_repository_intelligence::{
    AnalysisSnapshotId, Capability, CapabilityReport, CapabilityState, CodeEntityKind, Language,
    RepositorySnapshotId,
};

const RENDERED_BODY_CLAIM_LIMIT: usize = 12;
const RENDERED_METADATA_ITEM_LIMIT: usize = 8;
pub const RENDERED_DOCUMENT_FIELD_BYTE_LIMIT: usize = 4_096;
pub const RENDERED_MARKDOWN_BYTE_LIMIT: usize = 3 * 1_024 * 1_024;
pub const RENDERED_HTML_BYTE_LIMIT: usize = 8 * 1_024 * 1_024;

pub const GENERATED_DOCUMENT_FORMAT_KIND: &str = "volicord.generated_document";
pub const GENERATED_DOCUMENT_METADATA_VERSION: u32 = 4;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DocumentKind {
    ProjectArchitectureGuide,
    DecisionReport,
    ImplementationPlan,
    HandoffResume,
}

impl DocumentKind {
    pub const ALL: [Self; 4] = [
        Self::ProjectArchitectureGuide,
        Self::DecisionReport,
        Self::ImplementationPlan,
        Self::HandoffResume,
    ];

    pub const fn slug(self) -> &'static str {
        match self {
            Self::ProjectArchitectureGuide => "project-architecture-guide",
            Self::DecisionReport => "decision-report",
            Self::ImplementationPlan => "implementation-plan",
            Self::HandoffResume => "handoff-resume",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixedLocale {
    English,
    Korean,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratorIdentity {
    pub generator: String,
    pub agent: Option<String>,
    pub model: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OutputFormat {
    Markdown,
    Html,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestedDestination {
    pub document_kind: DocumentKind,
    pub output_format: OutputFormat,
    pub path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentRequest {
    /// Arbitrary user-provided generated-content language instruction. It is
    /// recorded exactly and never checked against a natural-language allowlist.
    pub requested_language: String,
    pub fixed_locale: FixedLocale,
    pub generated_at: TimestampMicros,
    pub generator: GeneratorIdentity,
    pub requested_destinations: Vec<RequestedDestination>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ClaimClass {
    CanonicalContext,
    RepositoryObservation,
    StructuralFact,
    SemanticResult,
    AgentInterpretation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedDocumentClaim {
    pub identity: String,
    pub class: ClaimClass,
    pub text: String,
    pub source_basis: Vec<SourceId>,
    pub decision_basis: Vec<DecisionId>,
    pub analysis_basis: Vec<AnalysisSnapshotId>,
    pub explicit_inference: bool,
    /// Historical ambiguity that was part of the displayed Question basis but
    /// is no longer a current uncertainty after a current Decision terminally
    /// resolved the choice.
    pub historical_uncertainty: Vec<String>,
    pub uncertainty: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentSection {
    pub key: String,
    pub title: String,
    pub claims: Vec<GeneratedDocumentClaim>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentBody {
    pub title: String,
    pub sections: Vec<DocumentSection>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentSourceBasis {
    pub source_id: SourceId,
    pub freshness: SourceFreshness,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentDecisionBasis {
    pub decision_id: DecisionId,
    pub revision: u64,
    pub state: BriefDecisionState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentMetadata {
    pub format_kind: String,
    pub format_version: u32,
    pub document_kind: DocumentKind,
    pub project_id: ProjectId,
    pub canonical_revision: u64,
    pub generated_at: TimestampMicros,
    pub generator: GeneratorIdentity,
    pub requested_language: String,
    /// Conservative normalized syntax metadata for the HTML `lang`
    /// attribute. This never replaces the generated-content language request.
    pub html_language_tag: String,
    pub fixed_locale: FixedLocale,
    pub repository_snapshots: Vec<RepositorySnapshotId>,
    pub analysis_snapshots: Vec<AnalysisSnapshotId>,
    pub included_decisions: Vec<DocumentDecisionBasis>,
    pub used_sources: Vec<DocumentSourceBasis>,
    pub capability_coverage: Vec<CapabilityReport>,
    pub capability_gaps: Vec<CapabilityGap>,
    pub omissions: Vec<ProjectionIssue>,
    pub requested_destination_basis: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicationArtifact {
    pub format: OutputFormat,
    pub media_type: String,
    pub suggested_file_name: String,
    pub requested_destination: Option<String>,
    pub content: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedDocument {
    pub metadata: DocumentMetadata,
    pub body: DocumentBody,
    pub markdown: PublicationArtifact,
    pub html: PublicationArtifact,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentSet {
    pub project_architecture_guide: GeneratedDocument,
    pub decision_report: GeneratedDocument,
    pub implementation_plan: GeneratedDocument,
    pub handoff_resume: GeneratedDocument,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentError {
    message: String,
}

impl DocumentError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for DocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl StdError for DocumentError {}

/// Generates all required document variants from one immutable semantic body
/// per document. It returns bounded publication artifacts and performs no I/O.
pub fn generate_documents(
    projection: &ProjectProjection,
    request: &DocumentRequest,
) -> Result<DocumentSet, DocumentError> {
    validate_request(request)?;
    let mut documents = BTreeMap::new();
    for kind in DocumentKind::ALL {
        let body = build_body(kind, projection, request.fixed_locale);
        validate_claim_grounding(projection, &body)?;
        let metadata = build_metadata(kind, projection, request, &body);
        let markdown_content = render_markdown(&metadata, &body, request.fixed_locale);
        let html_content = render_html(&metadata, &body, request.fixed_locale);
        validate_rendered_size(OutputFormat::Markdown, &markdown_content)?;
        validate_rendered_size(OutputFormat::Html, &html_content)?;
        let document = GeneratedDocument {
            markdown: PublicationArtifact {
                format: OutputFormat::Markdown,
                media_type: "text/markdown; charset=utf-8".to_owned(),
                suggested_file_name: format!("{}.md", kind.slug()),
                requested_destination: destination(request, kind, OutputFormat::Markdown),
                content: markdown_content,
            },
            html: PublicationArtifact {
                format: OutputFormat::Html,
                media_type: "text/html; charset=utf-8".to_owned(),
                suggested_file_name: format!("{}.html", kind.slug()),
                requested_destination: destination(request, kind, OutputFormat::Html),
                content: html_content,
            },
            metadata,
            body,
        };
        documents.insert(kind, document);
    }
    Ok(DocumentSet {
        project_architecture_guide: take_document(
            &mut documents,
            DocumentKind::ProjectArchitectureGuide,
        )?,
        decision_report: take_document(&mut documents, DocumentKind::DecisionReport)?,
        implementation_plan: take_document(&mut documents, DocumentKind::ImplementationPlan)?,
        handoff_resume: take_document(&mut documents, DocumentKind::HandoffResume)?,
    })
}

fn validate_request(request: &DocumentRequest) -> Result<(), DocumentError> {
    if request.requested_language.trim().is_empty() {
        return Err(DocumentError::new(
            "requested document language must not be empty",
        ));
    }
    if request.generator.generator.trim().is_empty() {
        return Err(DocumentError::new("generator identity must not be empty"));
    }
    let mut destinations = BTreeSet::new();
    for destination in &request.requested_destinations {
        if destination.path.trim().is_empty() {
            return Err(DocumentError::new(
                "an explicitly requested destination must not be empty",
            ));
        }
        if !destinations.insert((destination.document_kind, destination.output_format)) {
            return Err(DocumentError::new(
                "each document format may have at most one requested destination",
            ));
        }
    }
    Ok(())
}

fn validate_rendered_size(format: OutputFormat, content: &str) -> Result<(), DocumentError> {
    let limit = match format {
        OutputFormat::Markdown => RENDERED_MARKDOWN_BYTE_LIMIT,
        OutputFormat::Html => RENDERED_HTML_BYTE_LIMIT,
    };
    if content.len() > limit {
        return Err(DocumentError::new(format!(
            "bounded {} rendering exceeded its deterministic byte contract",
            output_format_key(format)
        )));
    }
    Ok(())
}

fn build_body(
    kind: DocumentKind,
    projection: &ProjectProjection,
    locale: FixedLocale,
) -> DocumentBody {
    let mut body = match kind {
        DocumentKind::ProjectArchitectureGuide => architecture_body(projection, locale),
        DocumentKind::DecisionReport => decision_body(projection, locale),
        DocumentKind::ImplementationPlan => implementation_body(projection, locale),
        DocumentKind::HandoffResume => handoff_body(projection, locale),
    };
    bound_rendered_body(&mut body, locale);
    body
}

fn architecture_body(projection: &ProjectProjection, locale: FixedLocale) -> DocumentBody {
    let mut overview_claims = projection
        .resume
        .goals_and_why
        .iter()
        .map(|goal| GeneratedDocumentClaim {
            identity: format!("context:{}", goal.identity),
            class: ClaimClass::CanonicalContext,
            text: goal.statement.clone(),
            source_basis: goal.source_basis.clone(),
            decision_basis: Vec::new(),
            analysis_basis: Vec::new(),
            explicit_inference: false,
            historical_uncertainty: Vec::new(),
            uncertainty: Vec::new(),
        })
        .collect::<Vec<_>>();
    if overview_claims.is_empty() {
        overview_claims.push(inference_claim(
            "project-goal-gap",
            fixed(
                locale,
                "Project goal is not recorded.",
                "프로젝트 목표가 기록되지 않았습니다.",
            ),
            Vec::new(),
        ));
    }

    let mut architecture_claims = projection
        .repository_map
        .entities
        .iter()
        .map(|entity| GeneratedDocumentClaim {
            identity: format!("entity:{}", entity.identity),
            class: ClaimClass::StructuralFact,
            text: format!(
                "{} ({}, {})",
                entity.display_name,
                code_entity_kind_label(&entity.kind, locale),
                language_label(&entity.language, locale)
            ),
            source_basis: vec![entity.source_id],
            decision_basis: Vec::new(),
            analysis_basis: vec![entity.analysis_snapshot],
            explicit_inference: false,
            historical_uncertainty: Vec::new(),
            uncertainty: entity.uncertainty.reasons.clone(),
        })
        .collect::<Vec<_>>();
    architecture_claims.extend(projection.repository_map.relations.iter().map(|relation| {
        let target = relation
            .target_entity
            .clone()
            .or_else(|| relation.unresolved_target.clone())
            .unwrap_or_else(|| "unresolved target".to_owned());
        GeneratedDocumentClaim {
            identity: format!("relation:{}", relation.identity),
            class: match relation.class {
                MapRelationClass::StructuralFact => ClaimClass::StructuralFact,
                MapRelationClass::SemanticResult => ClaimClass::SemanticResult,
            },
            text: format!(
                "{} --{}--> {}",
                relation.source_entity, relation.kind, target
            ),
            source_basis: vec![relation.source_id],
            decision_basis: Vec::new(),
            analysis_basis: vec![relation.analysis_snapshot],
            explicit_inference: false,
            historical_uncertainty: Vec::new(),
            uncertainty: relation
                .uncertainty
                .reasons
                .iter()
                .chain(&relation.diagnostics)
                .cloned()
                .collect(),
        }
    }));
    architecture_claims.extend(projection.repository_map.agent_interpretations.iter().map(
        |interpretation| {
            GeneratedDocumentClaim {
                identity: format!("interpretation:{}", interpretation.identity),
                class: ClaimClass::AgentInterpretation,
                text: interpretation.text.clone(),
                source_basis: interpretation.source_basis.clone(),
                decision_basis: Vec::new(),
                analysis_basis: vec![interpretation.analysis_snapshot],
                explicit_inference: true,
                historical_uncertainty: Vec::new(),
                uncertainty: interpretation
                    .uncertainty
                    .reasons
                    .iter()
                    .chain(&interpretation.known_gaps)
                    .cloned()
                    .collect(),
            }
        },
    ));
    if architecture_claims.is_empty() {
        architecture_claims.push(inference_claim(
            "architecture-gap",
            fixed(
                locale,
                "No structural or semantic architecture evidence is available; architecture interpretation is omitted.",
                "구조 또는 semantic architecture 근거를 사용할 수 없어 architecture 해석을 생략했습니다.",
            ),
            projection
                .source_catalog
                .iter()
                .map(|source| source.source.id)
                .take(1)
                .collect(),
        ));
    }
    let decision_claims = projection
        .resume
        .decisions
        .iter()
        .map(|decision| GeneratedDocumentClaim {
            identity: format!("decision:{}", decision.decision_id),
            class: ClaimClass::CanonicalContext,
            text: format!(
                "{}: {}; {}={}; {}={}",
                brief_decision_state_label(decision.state, locale),
                decision_choice_label(&decision.choice, locale),
                fixed(locale, "rationale", "근거"),
                decision.user_rationale.as_deref().unwrap_or_else(|| fixed(
                    locale,
                    "not recorded",
                    "기록되지 않음"
                )),
                fixed(locale, "applicability", "적용 범위"),
                projection
                    .decision_context_code
                    .iter()
                    .find(|link| link.decision_id == decision.decision_id)
                    .map_or_else(
                        || fixed(locale, "not recorded", "기록되지 않음").to_owned(),
                        |link| format_scope(&link.declared_paths, &link.declared_components)
                    )
            ),
            source_basis: decision.source_basis.clone(),
            decision_basis: vec![decision.decision_id],
            analysis_basis: Vec::new(),
            explicit_inference: false,
            historical_uncertainty: decision.question_uncertainty.clone(),
            uncertainty: decision
                .known_limits
                .iter()
                .chain(&decision.review_basis)
                .cloned()
                .collect(),
        })
        .collect();
    DocumentBody {
        title: fixed(
            locale,
            "Project & Architecture Guide",
            "프로젝트 및 아키텍처 가이드",
        )
        .to_owned(),
        sections: vec![
            section(
                "overview",
                fixed(locale, "Project overview", "프로젝트 개요"),
                overview_claims,
            ),
            section(
                "decisions",
                fixed(locale, "Architecture decisions", "아키텍처 결정"),
                decision_claims,
            ),
            timeline_section(projection, locale),
            section(
                "architecture",
                fixed(
                    locale,
                    "Repository architecture evidence",
                    "저장소 아키텍처 근거",
                ),
                architecture_claims,
            ),
            gap_section(projection, locale),
        ],
    }
}

fn decision_body(projection: &ProjectProjection, locale: FixedLocale) -> DocumentBody {
    let claims = projection
        .resume
        .decisions
        .iter()
        .map(|decision| {
            let link = projection
                .decision_context_code
                .iter()
                .find(|link| link.decision_id == decision.decision_id);
            GeneratedDocumentClaim {
                identity: format!("decision:{}", decision.decision_id),
                class: ClaimClass::CanonicalContext,
                text: format!(
                    "{}={}; {}={}; {}={}; {}={}; {}={}; {}={}; {}={}; {}={}; {}={}",
                    fixed(locale, "state", "상태"),
                    brief_decision_state_label(decision.state, locale),
                    fixed(locale, "choice", "선택"),
                    decision_choice_label(&decision.choice, locale),
                    fixed(locale, "user rationale", "사용자 근거"),
                    decision.user_rationale.as_deref().unwrap_or_else(|| fixed(
                        locale,
                        "not recorded",
                        "기록되지 않음"
                    )),
                    fixed(locale, "agent recommendation", "에이전트 권고"),
                    decision.recommendation_rationale,
                    fixed(locale, "expected consequence", "예상 결과"),
                    display_strings(&decision.expected_consequences, locale),
                    fixed(locale, "assumptions", "가정"),
                    display_strings(&decision.assumptions, locale),
                    fixed(locale, "revisit triggers", "재검토 조건"),
                    display_strings(&decision.revisit_triggers, locale),
                    fixed(locale, "scope", "범위"),
                    link.map_or_else(
                        || fixed(locale, "not recorded", "기록되지 않음").to_owned(),
                        |value| format!(
                            "{}; {}={}",
                            format_scope(&value.declared_paths, &value.declared_components),
                            fixed(locale, "work context", "작업 맥락"),
                            display_strings(&value.declared_work_contexts, locale)
                        )
                    ),
                    fixed(locale, "code links", "코드 연결"),
                    link.map_or_else(
                        || fixed(locale, "none", "없음").to_owned(),
                        |value| display_strings(&value.related_code_entities, locale)
                    ),
                ),
                source_basis: decision.source_basis.clone(),
                decision_basis: vec![decision.decision_id],
                analysis_basis: Vec::new(),
                explicit_inference: false,
                historical_uncertainty: decision.question_uncertainty.clone(),
                uncertainty: decision
                    .known_limits
                    .iter()
                    .chain(&decision.review_basis)
                    .chain(
                        link.into_iter()
                            .flat_map(|value| value.missing_or_uncertain_links.iter()),
                    )
                    .cloned()
                    .collect(),
            }
        })
        .collect();
    DocumentBody {
        title: fixed(locale, "Decision Report", "결정 보고서").to_owned(),
        sections: vec![
            goal_section(projection, locale),
            section(
                "decisions",
                fixed(
                    locale,
                    "Current Decision and consequence",
                    "현재 결정과 결과",
                ),
                claims,
            ),
            open_questions_section(projection, locale),
            gap_section(projection, locale),
        ],
    }
}

fn implementation_body(projection: &ProjectProjection, locale: FixedLocale) -> DocumentBody {
    let mut plan = Vec::new();
    for question in &projection.resume.open_questions {
        plan.push(GeneratedDocumentClaim {
            identity: format!("question:{}", question.question_id),
            class: ClaimClass::CanonicalContext,
            text: format!(
                "{} '{}' ({}={}, {}={}, {}={})",
                fixed(locale, "Resolve", "해결"),
                question.prompt,
                fixed(locale, "current frontier", "현재 프런티어"),
                yes_no(question.on_current_frontier, locale),
                fixed(locale, "unlocks", "해제되는 작업"),
                display_strings(&question.what_the_answer_unlocks, locale),
                fixed(locale, "blocked by", "차단 근거"),
                display_strings(&question.blocked_basis, locale)
            ),
            source_basis: question.source_basis.clone(),
            decision_basis: Vec::new(),
            analysis_basis: Vec::new(),
            explicit_inference: true,
            historical_uncertainty: Vec::new(),
            uncertainty: question.blocked_basis.clone(),
        });
    }
    if let Some(checkpoint) = projection.resume.latest_meaningful_checkpoint.as_ref() {
        plan.push(GeneratedDocumentClaim {
            identity: format!("checkpoint-next:{}", checkpoint.id),
            class: ClaimClass::CanonicalContext,
            text: format!(
                "{}: {}; {}={}; {}={}; {}={}",
                fixed(locale, "Next step", "다음 단계"),
                checkpoint.next_step,
                fixed(locale, "affected paths", "변경 경로"),
                display_strings(&checkpoint.changed_paths, locale),
                fixed(locale, "verification", "검증"),
                checkpoint
                    .verification
                    .iter()
                    .map(|fact| verification_fact_label(fact, locale))
                    .collect::<Vec<_>>()
                    .join("; "),
                fixed(locale, "known limits", "알려진 한계"),
                display_strings(&checkpoint.known_limits, locale)
            ),
            source_basis: checkpoint.source_basis.clone(),
            decision_basis: checkpoint.applied_decisions.clone(),
            analysis_basis: Vec::new(),
            explicit_inference: false,
            historical_uncertainty: Vec::new(),
            uncertainty: checkpoint.known_limits.clone(),
        });
    }
    if plan.is_empty() {
        plan.push(inference_claim(
            "implementation-plan-gap",
            fixed(
                locale,
                "No source-grounded next step or open Question is recorded.",
                "source-grounded 다음 단계나 열린 질문이 기록되지 않았습니다.",
            ),
            Vec::new(),
        ));
    }
    DocumentBody {
        title: fixed(locale, "Implementation Plan", "구현 계획").to_owned(),
        sections: vec![
            goal_section(projection, locale),
            decision_summary_section(projection, locale),
            section("plan", fixed(locale, "Ordered work", "작업 순서"), plan),
            timeline_section(projection, locale),
            gap_section(projection, locale),
        ],
    }
}

fn handoff_body(projection: &ProjectProjection, locale: FixedLocale) -> DocumentBody {
    let mut context = projection
        .resume
        .goals_and_why
        .iter()
        .map(|item| GeneratedDocumentClaim {
            identity: format!("goal:{}", item.identity),
            class: ClaimClass::CanonicalContext,
            text: item.statement.clone(),
            source_basis: item.source_basis.clone(),
            decision_basis: Vec::new(),
            analysis_basis: Vec::new(),
            explicit_inference: false,
            historical_uncertainty: Vec::new(),
            uncertainty: Vec::new(),
        })
        .collect::<Vec<_>>();
    context.extend(
        projection
            .resume
            .risks_assumptions_and_limits
            .iter()
            .map(|item| GeneratedDocumentClaim {
                identity: format!("context:{}", item.identity),
                class: ClaimClass::CanonicalContext,
                text: format!(
                    "{}: {}",
                    context_role_label(item.role, locale),
                    item.statement
                ),
                source_basis: item.source_basis.clone(),
                decision_basis: Vec::new(),
                analysis_basis: Vec::new(),
                explicit_inference: false,
                historical_uncertainty: Vec::new(),
                uncertainty: Vec::new(),
            }),
    );
    let questions = projection
        .resume
        .open_questions
        .iter()
        .map(|question| GeneratedDocumentClaim {
            identity: format!("question:{}", question.question_id),
            class: ClaimClass::CanonicalContext,
            text: format!(
                "{} ({}={}, {}={}, {}={})",
                question.prompt,
                fixed(locale, "current frontier", "현재 프런티어"),
                yes_no(question.on_current_frontier, locale),
                fixed(locale, "blocked by", "차단 근거"),
                display_strings(&question.blocked_basis, locale),
                fixed(locale, "unlocks", "해제되는 작업"),
                display_strings(&question.what_the_answer_unlocks, locale)
            ),
            source_basis: question.source_basis.clone(),
            decision_basis: Vec::new(),
            analysis_basis: Vec::new(),
            explicit_inference: false,
            historical_uncertainty: Vec::new(),
            uncertainty: question.blocked_basis.clone(),
        })
        .collect();
    DocumentBody {
        title: fixed(locale, "Handoff / Resume Document", "인계 / 재개 문서").to_owned(),
        sections: vec![
            section(
                "context",
                fixed(locale, "Goal and context", "목표와 맥락"),
                context,
            ),
            decision_summary_section(projection, locale),
            timeline_section(projection, locale),
            section(
                "questions",
                fixed(locale, "Open Questions", "열린 질문"),
                questions,
            ),
            section(
                "next-step",
                fixed(locale, "Next meaningful step", "다음 의미 있는 단계"),
                vec![GeneratedDocumentClaim {
                    identity: "next-meaningful-step".to_owned(),
                    class: ClaimClass::CanonicalContext,
                    text: projection
                        .resume
                        .next_meaningful_step
                        .clone()
                        .unwrap_or_else(|| {
                            fixed(locale, "Not recorded", "기록되지 않음").to_owned()
                        }),
                    source_basis: projection
                        .resume
                        .latest_meaningful_checkpoint
                        .as_ref()
                        .map(|checkpoint| checkpoint.source_basis.clone())
                        .unwrap_or_default(),
                    decision_basis: projection
                        .resume
                        .latest_meaningful_checkpoint
                        .as_ref()
                        .map(|checkpoint| checkpoint.applied_decisions.clone())
                        .unwrap_or_default(),
                    analysis_basis: Vec::new(),
                    explicit_inference: projection.resume.next_meaningful_step.is_none(),
                    historical_uncertainty: Vec::new(),
                    uncertainty: projection.resume.known_limits.clone(),
                }],
            ),
            gap_section(projection, locale),
        ],
    }
}

fn goal_section(projection: &ProjectProjection, locale: FixedLocale) -> DocumentSection {
    let mut claims = projection
        .resume
        .goals_and_why
        .iter()
        .map(|goal| GeneratedDocumentClaim {
            identity: format!("goal:{}", goal.identity),
            class: ClaimClass::CanonicalContext,
            text: goal.statement.clone(),
            source_basis: goal.source_basis.clone(),
            decision_basis: Vec::new(),
            analysis_basis: Vec::new(),
            explicit_inference: false,
            historical_uncertainty: Vec::new(),
            uncertainty: Vec::new(),
        })
        .collect::<Vec<_>>();
    if claims.is_empty() {
        claims.push(inference_claim(
            "project-goal-gap",
            fixed(
                locale,
                "Project goal is not recorded.",
                "프로젝트 목표가 기록되지 않았습니다.",
            ),
            Vec::new(),
        ));
    }
    section("goal", fixed(locale, "Current Goal", "현재 목표"), claims)
}

fn decision_summary_section(
    projection: &ProjectProjection,
    locale: FixedLocale,
) -> DocumentSection {
    let claims = projection
        .resume
        .decisions
        .iter()
        .filter(|decision| decision.state != BriefDecisionState::Superseded)
        .map(|decision| GeneratedDocumentClaim {
            identity: format!("decision-summary:{}", decision.decision_id),
            class: ClaimClass::CanonicalContext,
            text: format!(
                "{}: {}; {}={}; {}={}",
                fixed(locale, "Decision", "결정"),
                decision_choice_label(&decision.choice, locale),
                fixed(locale, "rationale", "근거"),
                decision.user_rationale.as_deref().unwrap_or_else(|| fixed(
                    locale,
                    "not recorded",
                    "기록되지 않음"
                )),
                fixed(locale, "consequence", "결과"),
                display_strings(&decision.expected_consequences, locale),
            ),
            source_basis: decision.source_basis.clone(),
            decision_basis: vec![decision.decision_id],
            analysis_basis: Vec::new(),
            explicit_inference: false,
            historical_uncertainty: decision.question_uncertainty.clone(),
            uncertainty: decision
                .known_limits
                .iter()
                .chain(&decision.review_basis)
                .cloned()
                .collect(),
        })
        .collect();
    section(
        "decision-summary",
        fixed(locale, "Current Decision direction", "현재 결정 방향"),
        claims,
    )
}

fn open_questions_section(projection: &ProjectProjection, locale: FixedLocale) -> DocumentSection {
    let claims = projection
        .resume
        .open_questions
        .iter()
        .map(|question| GeneratedDocumentClaim {
            identity: format!("question:{}", question.question_id),
            class: ClaimClass::CanonicalContext,
            text: format!(
                "{}; {}={}; {}={}",
                question.prompt,
                fixed(locale, "unlocks", "해제되는 작업"),
                display_strings(&question.what_the_answer_unlocks, locale),
                fixed(locale, "blocked by", "차단 근거"),
                display_strings(&question.blocked_basis, locale),
            ),
            source_basis: question.source_basis.clone(),
            decision_basis: Vec::new(),
            analysis_basis: Vec::new(),
            explicit_inference: false,
            historical_uncertainty: Vec::new(),
            uncertainty: question.blocked_basis.clone(),
        })
        .collect();
    section(
        "open-questions",
        fixed(locale, "Open material Questions", "열린 주요 질문"),
        claims,
    )
}

fn timeline_section(projection: &ProjectProjection, locale: FixedLocale) -> DocumentSection {
    let claims = projection
        .checkpoint_timeline
        .iter()
        .map(|entry| GeneratedDocumentClaim {
            identity: format!("checkpoint:{}", entry.checkpoint.id),
            class: ClaimClass::CanonicalContext,
            text: format!(
                "{}={}; {}={}; {}={}; {}={}; {}={}; {}={}; {}={}",
                fixed(locale, "goal", "목표"),
                entry.checkpoint.goal,
                fixed(locale, "work", "작업"),
                work_state_label(entry.work_state, locale),
                fixed(locale, "verification", "검증"),
                entry
                    .verification
                    .iter()
                    .map(|fact| verification_fact_label(fact, locale))
                    .collect::<Vec<_>>()
                    .join("; "),
                fixed(locale, "user review", "사용자 검토"),
                user_review_label(entry.user_review.state, locale),
                fixed(locale, "user acceptance", "사용자 수락"),
                user_acceptance_label(entry.user_acceptance.state, locale),
                fixed(locale, "changes", "변경"),
                display_strings(&entry.checkpoint.changed_paths, locale),
                fixed(locale, "next", "다음 단계"),
                entry.checkpoint.next_step,
            ),
            source_basis: entry.checkpoint.source_basis.clone(),
            decision_basis: entry.checkpoint.applied_decisions.clone(),
            analysis_basis: Vec::new(),
            explicit_inference: false,
            historical_uncertainty: Vec::new(),
            uncertainty: entry.checkpoint.known_limits.clone(),
        })
        .collect();
    section(
        "timeline",
        fixed(locale, "Checkpoint timeline", "체크포인트 타임라인"),
        claims,
    )
}

fn gap_section(projection: &ProjectProjection, locale: FixedLocale) -> DocumentSection {
    let mut claims = projection
        .repository_map
        .gaps
        .iter()
        .map(|gap| GeneratedDocumentClaim {
            identity: format!(
                "gap:{}:{}:{}:{}",
                gap.analysis_snapshot,
                capability_key(gap.capability),
                gap.language
                    .as_ref()
                    .map_or("all_languages".to_owned(), language_key),
                gap.area
            ),
            class: ClaimClass::RepositoryObservation,
            text: format!(
                "{}={}; {}={}; {}={}; {}={}; {}={}; {}={}; {}={}",
                fixed(locale, "state", "상태"),
                capability_state_label(gap.state, locale),
                fixed(locale, "capability", "기능"),
                capability_label(gap.capability, locale),
                fixed(locale, "language", "언어"),
                optional_language_label(gap.language.as_ref(), locale),
                fixed(locale, "area", "영역"),
                gap.area,
                fixed(locale, "reason", "이유"),
                gap.reason,
                fixed(locale, "affected", "영향 범위"),
                display_strings(&gap.affected_areas, locale),
                fixed(locale, "usable remainder", "사용 가능한 나머지"),
                gap.usable_remainder.as_deref().unwrap_or_else(|| fixed(
                    locale,
                    "not reported",
                    "보고되지 않음"
                ))
            ),
            source_basis: projection
                .source_catalog
                .iter()
                .filter(|source| source.snapshot_basis.is_some())
                .map(|source| source.source.id)
                .take(1)
                .collect(),
            decision_basis: Vec::new(),
            analysis_basis: vec![gap.analysis_snapshot],
            explicit_inference: false,
            historical_uncertainty: Vec::new(),
            uncertainty: vec![gap.reason.clone()],
        })
        .collect::<Vec<_>>();
    claims.extend(projection.issues.iter().map(|issue| {
        GeneratedDocumentClaim {
            identity: format!("omission:{}:{}", issue.affected_scope, issue.identity),
            class: ClaimClass::RepositoryObservation,
            text: format!(
                "{}: {} ({}={}; {}={})",
                issue.affected_scope,
                issue.reason,
                fixed(locale, "kind", "종류"),
                projection_issue_kind_label(issue.kind, locale),
                fixed(locale, "omitted count", "생략 수"),
                issue.omitted_count
            ),
            source_basis: projection
                .source_catalog
                .iter()
                .find(|source| source.source.id.to_string() == issue.identity)
                .map(|source| vec![source.source.id])
                .unwrap_or_default(),
            decision_basis: Vec::new(),
            analysis_basis: Vec::new(),
            explicit_inference: true,
            historical_uncertainty: Vec::new(),
            uncertainty: vec![issue.reason.clone()],
        }
    }));
    claims.sort_by(|left, right| {
        let left_priority = usize::from(
            !left
                .identity
                .starts_with("omission:candidate_inspection:candidate_dependency:"),
        );
        let right_priority = usize::from(
            !right
                .identity
                .starts_with("omission:candidate_inspection:candidate_dependency:"),
        );
        (left_priority, &left.identity).cmp(&(right_priority, &right.identity))
    });
    claims.dedup_by(|left, right| left.identity == right.identity);
    section(
        "gaps",
        fixed(
            locale,
            "Coverage, gaps, and omissions",
            "Coverage, 빈틈 및 누락",
        ),
        claims,
    )
}

fn inference_claim(
    identity: impl Into<String>,
    text: impl Into<String>,
    source_basis: Vec<SourceId>,
) -> GeneratedDocumentClaim {
    GeneratedDocumentClaim {
        identity: identity.into(),
        class: ClaimClass::AgentInterpretation,
        text: text.into(),
        source_basis,
        decision_basis: Vec::new(),
        analysis_basis: Vec::new(),
        explicit_inference: true,
        historical_uncertainty: Vec::new(),
        uncertainty: vec!["explicit inference or missing-basis marker".to_owned()],
    }
}

fn section(key: &str, title: &str, claims: Vec<GeneratedDocumentClaim>) -> DocumentSection {
    DocumentSection {
        key: key.to_owned(),
        title: title.to_owned(),
        claims,
    }
}

fn bound_rendered_body(body: &mut DocumentBody, locale: FixedLocale) {
    for section in &mut body.sections {
        if section.claims.len() <= RENDERED_BODY_CLAIM_LIMIT {
            continue;
        }
        let omitted_count = section.claims.len() - RENDERED_BODY_CLAIM_LIMIT;
        section.claims.truncate(RENDERED_BODY_CLAIM_LIMIT);
        section.claims.push(inference_claim(
            format!("render-bound:{}", section.key),
            format!(
                "{} {} {}.",
                omitted_count,
                fixed(
                    locale,
                    "additional grounded items are omitted from this rendered section",
                    "개의 추가 grounded 항목을 이 렌더링 섹션에서 생략했습니다"
                ),
                fixed(
                    locale,
                    "Full typed grounding remains available to internal callers",
                    "전체 typed grounding은 내부 호출자에게 계속 제공됩니다"
                )
            ),
            Vec::new(),
        ));
    }
}

fn validate_claim_grounding(
    projection: &ProjectProjection,
    body: &DocumentBody,
) -> Result<(), DocumentError> {
    let valid_sources = projection
        .source_catalog
        .iter()
        .map(|source| source.source.id)
        .collect::<BTreeSet<_>>();
    let valid_decisions = projection
        .resume
        .decisions
        .iter()
        .map(|decision| decision.decision_id)
        .collect::<BTreeSet<_>>();
    let valid_analyses = projection
        .resume
        .snapshots
        .iter()
        .map(|snapshot| snapshot.analysis_snapshot)
        .chain(
            projection
                .repository_map
                .entities
                .iter()
                .map(|entity| entity.analysis_snapshot),
        )
        .collect::<BTreeSet<_>>();
    for claim in body.sections.iter().flat_map(|section| &section.claims) {
        if claim.source_basis.is_empty()
            && claim.decision_basis.is_empty()
            && claim.analysis_basis.is_empty()
            && !claim.explicit_inference
        {
            return Err(DocumentError::new(format!(
                "claim {} has no Source basis or explicit inference marker",
                claim.identity
            )));
        }
        if let Some(source) = claim
            .source_basis
            .iter()
            .find(|source| !valid_sources.contains(source))
        {
            return Err(DocumentError::new(format!(
                "claim {} refers to unknown Source {}",
                claim.identity, source
            )));
        }
        if let Some(decision) = claim
            .decision_basis
            .iter()
            .find(|decision| !valid_decisions.contains(decision))
        {
            return Err(DocumentError::new(format!(
                "claim {} refers to omitted Decision {}",
                claim.identity, decision
            )));
        }
        if let Some(analysis) = claim
            .analysis_basis
            .iter()
            .find(|analysis| !valid_analyses.contains(analysis))
        {
            return Err(DocumentError::new(format!(
                "claim {} refers to unknown Analysis Snapshot {}",
                claim.identity, analysis
            )));
        }
    }
    Ok(())
}

fn build_metadata(
    kind: DocumentKind,
    projection: &ProjectProjection,
    request: &DocumentRequest,
    body: &DocumentBody,
) -> DocumentMetadata {
    let mut repository_snapshots = projection
        .resume
        .snapshots
        .iter()
        .map(|snapshot| snapshot.repository_snapshot)
        .chain(
            projection
                .repository_map
                .entities
                .iter()
                .map(|entity| entity.repository_snapshot),
        )
        .chain(
            projection
                .repository_map
                .relations
                .iter()
                .map(|relation| relation.repository_snapshot),
        )
        .chain(
            projection
                .repository_map
                .gaps
                .iter()
                .map(|gap| gap.repository_snapshot),
        )
        .chain(
            projection
                .repository_map
                .capabilities
                .iter()
                .map(|report| report.repository_snapshot),
        )
        .collect::<Vec<_>>();
    repository_snapshots.sort();
    repository_snapshots.dedup();
    let mut analysis_snapshots = body
        .sections
        .iter()
        .flat_map(|section| &section.claims)
        .flat_map(|claim| claim.analysis_basis.iter().copied())
        .chain(
            projection
                .resume
                .snapshots
                .iter()
                .map(|snapshot| snapshot.analysis_snapshot),
        )
        .chain(
            projection
                .repository_map
                .entities
                .iter()
                .map(|entity| entity.analysis_snapshot),
        )
        .chain(
            projection
                .repository_map
                .relations
                .iter()
                .map(|relation| relation.analysis_snapshot),
        )
        .chain(
            projection
                .repository_map
                .gaps
                .iter()
                .map(|gap| gap.analysis_snapshot),
        )
        .collect::<Vec<_>>();
    analysis_snapshots.sort();
    analysis_snapshots.dedup();
    let mut used_source_ids = body
        .sections
        .iter()
        .flat_map(|section| &section.claims)
        .flat_map(|claim| claim.source_basis.iter().copied())
        .collect::<Vec<_>>();
    used_source_ids.sort();
    used_source_ids.dedup();
    let source_freshness = projection
        .source_catalog
        .iter()
        .map(|source| (source.source.id, source.freshness))
        .collect::<BTreeMap<_, _>>();
    let used_sources = used_source_ids
        .into_iter()
        .map(|source_id| DocumentSourceBasis {
            source_id,
            freshness: source_freshness
                .get(&source_id)
                .copied()
                .unwrap_or(SourceFreshness::Unknown),
        })
        .collect();
    let included_decisions = projection
        .resume
        .decisions
        .iter()
        .map(|decision| DocumentDecisionBasis {
            decision_id: decision.decision_id,
            revision: decision.revision,
            state: decision.state,
        })
        .collect();
    let requested_destination_basis = request
        .requested_destinations
        .iter()
        .filter(|destination| destination.document_kind == kind)
        .map(|destination| {
            format!(
                "{}:{}",
                output_format_key(destination.output_format),
                destination.path
            )
        })
        .collect();
    DocumentMetadata {
        format_kind: GENERATED_DOCUMENT_FORMAT_KIND.to_owned(),
        format_version: GENERATED_DOCUMENT_METADATA_VERSION,
        document_kind: kind,
        project_id: projection.overview.project_id,
        canonical_revision: projection.overview.canonical_revision,
        generated_at: request.generated_at,
        generator: request.generator.clone(),
        requested_language: request.requested_language.clone(),
        html_language_tag: normalized_html_language_tag(
            &request.requested_language,
            request.fixed_locale,
        ),
        fixed_locale: request.fixed_locale,
        repository_snapshots,
        analysis_snapshots,
        included_decisions,
        used_sources,
        capability_coverage: projection.repository_map.capabilities.clone(),
        capability_gaps: projection.repository_map.gaps.clone(),
        omissions: projection.issues.clone(),
        requested_destination_basis,
    }
}

fn render_markdown(
    metadata: &DocumentMetadata,
    body: &DocumentBody,
    locale: FixedLocale,
) -> String {
    let mut output = String::new();
    output.push_str("# ");
    output.push_str(&escape_markdown(&bounded_rendered_field(
        &body.title,
        "document title",
        locale,
    )));
    output.push_str("\n\n");
    for section in &body.sections {
        output.push_str("## ");
        output.push_str(&escape_markdown(&bounded_rendered_field(
            &section.title,
            "section title",
            locale,
        )));
        output.push_str("\n\n");
        if section.claims.is_empty() {
            output.push_str(fixed(locale, "No grounded items.", "grounded 항목 없음."));
            output.push_str("\n\n");
        }
        for claim in &section.claims {
            output.push_str("- ");
            if claim.explicit_inference {
                output.push_str("**[");
                output.push_str(fixed(locale, "Inference", "추론"));
                output.push_str("]** ");
            }
            output.push_str(&escape_markdown(&bounded_rendered_field(
                &claim.text,
                "claim text",
                locale,
            )));
            if !claim.uncertainty.is_empty() {
                output.push_str("  \n  ");
                output.push_str(fixed(
                    locale,
                    "Known limit or uncertainty: ",
                    "알려진 한계 또는 불확실성: ",
                ));
                output.push_str(&escape_markdown(&bounded_rendered_field(
                    &claim.uncertainty.join("; "),
                    "claim uncertainty",
                    locale,
                )));
            }
            output.push('\n');
        }
        output.push('\n');
    }
    render_metadata_markdown(&mut output, metadata, body, locale);
    output
}

fn render_html(metadata: &DocumentMetadata, body: &DocumentBody, locale: FixedLocale) -> String {
    let mut output = String::from("<!doctype html><html lang=\"");
    output.push_str(&metadata.html_language_tag);
    output.push_str(
        "\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><style>body{font-family:system-ui,sans-serif;max-width:72rem;margin:2rem auto;padding:0 1rem;line-height:1.5;color:#17202a}dl{display:grid;grid-template-columns:max-content 1fr;gap:.25rem 1rem}dt{font-weight:700}.claim{border-left:.3rem solid #667085;padding:.6rem 1rem;margin:.75rem 0;background:#f8fafc}.basis,.uncertainty,.audit-meta{color:#475467;font-size:.92rem}.inference{font-weight:700;color:#9a3412}.audit{margin-top:2rem;border-top:1px solid #d0d5dd;padding-top:1rem}.audit>summary{cursor:pointer;font-weight:700}.audit-claim{padding:.5rem 0;border-top:1px solid #e4e7ec}code{overflow-wrap:anywhere}</style><title>",
    );
    output.push_str(&escape_html(&bounded_rendered_field(
        &body.title,
        "document title",
        locale,
    )));
    output.push_str("</title></head><body><main><h1>");
    output.push_str(&escape_html(&bounded_rendered_field(
        &body.title,
        "document title",
        locale,
    )));
    output.push_str("</h1>");
    for section in &body.sections {
        output.push_str("<section data-section=\"");
        output.push_str(&escape_html(&bounded_rendered_field(
            &section.key,
            "section identity",
            locale,
        )));
        output.push_str("\"><h2>");
        output.push_str(&escape_html(&bounded_rendered_field(
            &section.title,
            "section title",
            locale,
        )));
        output.push_str("</h2>");
        if section.claims.is_empty() {
            output.push_str("<p>");
            output.push_str(&escape_html(fixed(
                locale,
                "No grounded items.",
                "grounded 항목 없음.",
            )));
            output.push_str("</p>");
        }
        for claim in &section.claims {
            output.push_str("<article class=\"claim\">");
            if claim.explicit_inference {
                output.push_str("<span class=\"inference\">[");
                output.push_str(&escape_html(fixed(locale, "Inference", "추론")));
                output.push_str("]</span> ");
            }
            output.push_str(&escape_html(&bounded_rendered_field(
                &claim.text,
                "claim text",
                locale,
            )));
            if !claim.uncertainty.is_empty() {
                output.push_str("<div class=\"uncertainty\"><strong>");
                output.push_str(&escape_html(fixed(
                    locale,
                    "Known limit or uncertainty:",
                    "알려진 한계 또는 불확실성:",
                )));
                output.push_str("</strong> ");
                output.push_str(&escape_html(&bounded_rendered_field(
                    &claim.uncertainty.join("; "),
                    "claim uncertainty",
                    locale,
                )));
                output.push_str("</div>");
            }
            output.push_str("</article>");
        }
        output.push_str("</section>");
    }
    render_metadata_html(&mut output, metadata, body, locale);
    output.push_str("</main></body></html>\n");
    output
}

fn render_metadata_markdown(
    output: &mut String,
    metadata: &DocumentMetadata,
    body: &DocumentBody,
    locale: FixedLocale,
) {
    output.push_str("## ");
    output.push_str(fixed(
        locale,
        "Grounding and audit appendix",
        "Grounding 및 감사 부록",
    ));
    output.push_str("\n\n");
    output.push_str(fixed(locale, "This trailing appendix preserves the bounded technical basis for the human-facing document above.\n\n", "이 후행 부록은 위의 사용자 중심 문서에 대한 범위 제한 기술 근거를 보존합니다.\n\n"));
    for (label, value) in metadata_pairs(metadata, locale) {
        output.push_str("- **");
        output.push_str(label);
        output.push_str(":** ");
        output.push_str(&escape_markdown(&bounded_rendered_field(
            &value,
            "metadata value",
            locale,
        )));
        output.push('\n');
    }
    output.push('\n');
    output.push_str("### ");
    output.push_str(fixed(locale, "Direct claim basis", "직접 주장 근거"));
    output.push_str("\n\n");
    for claim in body.sections.iter().flat_map(|section| &section.claims) {
        output.push_str("- **");
        output.push_str(&escape_markdown(&bounded_rendered_field(
            &claim.identity,
            "claim identity",
            locale,
        )));
        output.push_str(":** ");
        output.push_str(claim_class_label(claim.class, locale));
        output.push_str("; ");
        output.push_str(&escape_markdown(&bounded_rendered_field(
            &claim_basis(claim),
            "claim basis",
            locale,
        )));
        if !claim.historical_uncertainty.is_empty() {
            output.push_str("; ");
            output.push_str(fixed(
                locale,
                "resolved Question ambiguity=",
                "해결된 Question 모호성=",
            ));
            output.push_str(&escape_markdown(&bounded_rendered_field(
                &claim.historical_uncertainty.join("; "),
                "historical claim uncertainty",
                locale,
            )));
        }
        output.push('\n');
    }
    output.push('\n');
}

fn render_metadata_html(
    output: &mut String,
    metadata: &DocumentMetadata,
    body: &DocumentBody,
    locale: FixedLocale,
) {
    output.push_str("<details class=\"audit\" data-section=\"grounding-audit\"><summary>");
    output.push_str(&escape_html(fixed(
        locale,
        "Grounding and audit appendix",
        "Grounding 및 감사 부록",
    )));
    output.push_str("</summary><p class=\"audit-meta\">");
    output.push_str(&escape_html(fixed(
        locale,
        "Bounded technical basis for this document.",
        "이 문서의 범위 제한 기술 근거입니다.",
    )));
    output.push_str("</p><dl>");
    for (label, value) in metadata_pairs(metadata, locale) {
        output.push_str("<dt>");
        output.push_str(&escape_html(label));
        output.push_str("</dt><dd>");
        output.push_str(&escape_html(&bounded_rendered_field(
            &value,
            "metadata value",
            locale,
        )));
        output.push_str("</dd>");
    }
    output.push_str("</dl><h3>");
    output.push_str(&escape_html(fixed(
        locale,
        "Direct claim basis",
        "직접 주장 근거",
    )));
    output.push_str("</h3>");
    for claim in body.sections.iter().flat_map(|section| &section.claims) {
        output.push_str("<article class=\"audit-claim\" data-claim-id=\"");
        output.push_str(&escape_html(&bounded_rendered_field(
            &claim.identity,
            "claim identity",
            locale,
        )));
        output.push_str("\"><strong>");
        output.push_str(&escape_html(&bounded_rendered_field(
            &claim.identity,
            "claim identity",
            locale,
        )));
        output.push_str("</strong> · ");
        output.push_str(claim_class_label(claim.class, locale));
        output.push_str("<div class=\"basis\">");
        output.push_str(&escape_html(&bounded_rendered_field(
            &claim_basis(claim),
            "claim basis",
            locale,
        )));
        output.push_str("</div>");
        if !claim.historical_uncertainty.is_empty() {
            output.push_str("<div class=\"uncertainty\"><strong>");
            output.push_str(&escape_html(fixed(
                locale,
                "Resolved Question ambiguity:",
                "해결된 Question 모호성:",
            )));
            output.push_str("</strong> ");
            output.push_str(&escape_html(&bounded_rendered_field(
                &claim.historical_uncertainty.join("; "),
                "historical claim uncertainty",
                locale,
            )));
            output.push_str("</div>");
        }
        output.push_str("</article>");
    }
    output.push_str("</details>");
}

fn metadata_pairs(metadata: &DocumentMetadata, locale: FixedLocale) -> Vec<(&'static str, String)> {
    let repository_snapshots = metadata
        .repository_snapshots
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let analysis_snapshots = metadata
        .analysis_snapshots
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let decisions = metadata
        .included_decisions
        .iter()
        .map(|decision| {
            format!(
                "{}@{}:{}",
                decision.decision_id,
                decision.revision,
                brief_decision_state_label(decision.state, locale)
            )
        })
        .collect::<Vec<_>>();
    let sources = metadata
        .used_sources
        .iter()
        .map(|source| {
            format!(
                "{}:{}",
                source.source_id,
                source_freshness_label(source.freshness, locale)
            )
        })
        .collect::<Vec<_>>();
    vec![
        (
            fixed(locale, "format", "형식"),
            format!("{}@{}", metadata.format_kind, metadata.format_version),
        ),
        (
            fixed(locale, "document", "문서"),
            document_kind_label(metadata.document_kind, locale).to_owned(),
        ),
        (
            fixed(locale, "project", "프로젝트"),
            metadata.project_id.to_string(),
        ),
        (
            fixed(locale, "canonical revision", "정식 리비전"),
            metadata.canonical_revision.to_string(),
        ),
        (
            fixed(locale, "generated at", "생성 시각"),
            metadata.generated_at.as_unix_micros().to_string(),
        ),
        (
            fixed(locale, "generator", "생성기"),
            format!(
                "{}; agent={}; model={}",
                metadata.generator.generator,
                metadata.generator.agent.as_deref().unwrap_or("—"),
                metadata.generator.model.as_deref().unwrap_or("—")
            ),
        ),
        (
            fixed(locale, "requested language", "요청 언어"),
            metadata.requested_language.clone(),
        ),
        (
            fixed(locale, "HTML language tag", "HTML 언어 태그"),
            metadata.html_language_tag.clone(),
        ),
        (
            fixed(locale, "repository snapshots", "저장소 스냅샷"),
            bounded_rendered_list(&repository_snapshots, locale),
        ),
        (
            fixed(locale, "analysis snapshots", "분석 스냅샷"),
            bounded_rendered_list(&analysis_snapshots, locale),
        ),
        (
            fixed(locale, "included Decisions", "포함된 결정"),
            bounded_rendered_list(&decisions, locale),
        ),
        (
            fixed(locale, "used Sources", "사용한 Source"),
            bounded_rendered_list(&sources, locale),
        ),
        (
            fixed(locale, "capability coverage", "기능 범위"),
            coverage_summary(&metadata.capability_coverage, locale),
        ),
        (
            fixed(locale, "known gaps", "알려진 빈틈"),
            gap_summary(&metadata.capability_gaps, locale),
        ),
        (
            fixed(locale, "omissions", "생략"),
            omission_summary(&metadata.omissions, locale),
        ),
        (
            fixed(locale, "requested destinations", "요청 대상"),
            bounded_rendered_list(&metadata.requested_destination_basis, locale),
        ),
    ]
}

fn claim_basis(claim: &GeneratedDocumentClaim) -> String {
    format!(
        "claim={}; sources=[{}]; decisions=[{}]; analyses=[{}]",
        claim.identity,
        join_display(&claim.source_basis),
        join_display(&claim.decision_basis),
        join_display(&claim.analysis_basis)
    )
}

fn destination(
    request: &DocumentRequest,
    kind: DocumentKind,
    format: OutputFormat,
) -> Option<String> {
    request
        .requested_destinations
        .iter()
        .find(|destination| {
            destination.document_kind == kind && destination.output_format == format
        })
        .map(|destination| destination.path.clone())
}

const fn output_format_key(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Markdown => "markdown",
        OutputFormat::Html => "html",
    }
}

fn take_document(
    values: &mut BTreeMap<DocumentKind, GeneratedDocument>,
    kind: DocumentKind,
) -> Result<GeneratedDocument, DocumentError> {
    values
        .remove(&kind)
        .ok_or_else(|| DocumentError::new("required generated document is missing"))
}

fn join_display<T: fmt::Display>(values: &[T]) -> String {
    values
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn bounded_rendered_field(value: &str, field: &str, locale: FixedLocale) -> String {
    if value.len() <= RENDERED_DOCUMENT_FIELD_BYTE_LIMIT {
        return value.to_owned();
    }
    format!(
        "[{}: {}; {}={}; {}={}]",
        fixed(locale, "omitted oversized field", "크기 초과 필드 생략"),
        field,
        fixed(locale, "exact UTF-8 bytes", "정확한 UTF-8 바이트"),
        value.len(),
        fixed(locale, "rendered byte limit", "렌더링 바이트 제한"),
        RENDERED_DOCUMENT_FIELD_BYTE_LIMIT
    )
}

fn normalized_html_language_tag(requested: &str, locale: FixedLocale) -> String {
    let fallback = match locale {
        FixedLocale::English => "en",
        FixedLocale::Korean => "ko",
    };
    let candidate = requested.trim().replace('_', "-");
    if candidate.is_empty() || candidate.len() > 63 || !candidate.is_ascii() {
        return fallback.to_owned();
    }
    let subtags = candidate.split('-').collect::<Vec<_>>();
    let Some(language) = subtags.first() else {
        return fallback.to_owned();
    };
    if !(2..=8).contains(&language.len())
        || !language.bytes().all(|byte| byte.is_ascii_alphabetic())
    {
        return fallback.to_owned();
    }
    if subtags.iter().skip(1).any(|subtag| {
        subtag.is_empty()
            || subtag.len() > 8
            || !subtag.bytes().all(|byte| byte.is_ascii_alphanumeric())
    }) || subtags.last().is_some_and(|subtag| subtag.len() == 1)
    {
        return fallback.to_owned();
    }

    let mut normalized = Vec::with_capacity(subtags.len());
    normalized.push(language.to_ascii_lowercase());
    for subtag in subtags.into_iter().skip(1) {
        let value = if subtag.len() == 4 && subtag.bytes().all(|byte| byte.is_ascii_alphabetic()) {
            let mut value = subtag.to_ascii_lowercase();
            value.replace_range(0..1, &subtag[0..1].to_ascii_uppercase());
            value
        } else if (subtag.len() == 2 && subtag.bytes().all(|byte| byte.is_ascii_alphabetic()))
            || (subtag.len() == 3 && subtag.bytes().all(|byte| byte.is_ascii_digit()))
        {
            subtag.to_ascii_uppercase()
        } else {
            subtag.to_ascii_lowercase()
        };
        normalized.push(value);
    }
    normalized.join("-")
}

fn bounded_rendered_list(values: &[String], locale: FixedLocale) -> String {
    if values.is_empty() {
        return fixed(locale, "none", "없음").to_owned();
    }
    let mut rendered = values
        .iter()
        .take(RENDERED_METADATA_ITEM_LIMIT)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let omitted = values.len().saturating_sub(RENDERED_METADATA_ITEM_LIMIT);
    if omitted > 0 {
        rendered.push_str(&format!(
            "; {} {}",
            omitted,
            fixed(
                locale,
                "additional items omitted from rendered metadata",
                "개 추가 항목을 렌더링 메타데이터에서 생략"
            )
        ));
    }
    rendered
}

fn coverage_summary(reports: &[CapabilityReport], locale: FixedLocale) -> String {
    let values = reports
        .iter()
        .map(|report| {
            format!(
                "{}/{}/{}={} files:{} entities:{} relations:{}",
                optional_language_label(report.language.as_ref(), locale),
                capability_label(report.capability, locale),
                report.area.path,
                capability_state_label(report.state, locale),
                report.coverage.covered_file_count,
                report.coverage.covered_entity_count,
                report.coverage.covered_relation_count
            )
        })
        .collect::<Vec<_>>();
    bounded_rendered_list(&values, locale)
}

fn gap_summary(gaps: &[CapabilityGap], locale: FixedLocale) -> String {
    let values = gaps
        .iter()
        .map(|gap| {
            format!(
                "{}/{}/{}={}: {}",
                optional_language_label(gap.language.as_ref(), locale),
                capability_label(gap.capability, locale),
                gap.area,
                capability_state_label(gap.state, locale),
                gap.reason
            )
        })
        .collect::<Vec<_>>();
    bounded_rendered_list(&values, locale)
}

fn omission_summary(issues: &[ProjectionIssue], locale: FixedLocale) -> String {
    let bounded = issues
        .iter()
        .filter(|issue| issue.omitted_count > 0)
        .map(|issue| {
            format!(
                "{}: {}={}",
                issue.affected_scope,
                fixed(locale, "exact omitted count", "정확한 생략 수"),
                issue.omitted_count
            )
        })
        .collect::<Vec<_>>();
    let mut other_counts = BTreeMap::<&'static str, usize>::new();
    for issue in issues.iter().filter(|issue| issue.omitted_count == 0) {
        *other_counts
            .entry(projection_issue_kind_label(issue.kind, locale))
            .or_default() += 1;
    }
    let mut values = bounded;
    values.extend(other_counts.into_iter().map(|(kind, count)| {
        format!(
            "{}: {} {}",
            kind,
            count,
            fixed(locale, "reported issues", "건의 보고된 문제")
        )
    }));
    if values.is_empty() {
        return fixed(locale, "none", "없음").to_owned();
    }
    values.join("; ")
}

const fn claim_class_label(class: ClaimClass, locale: FixedLocale) -> &'static str {
    match class {
        ClaimClass::CanonicalContext => fixed(locale, "Canonical Context", "정식 맥락"),
        ClaimClass::RepositoryObservation => fixed(locale, "Repository Observation", "저장소 관찰"),
        ClaimClass::StructuralFact => fixed(locale, "Structural Fact", "구조 사실"),
        ClaimClass::SemanticResult => fixed(locale, "Semantic Result", "의미 분석 결과"),
        ClaimClass::AgentInterpretation => fixed(locale, "Agent Interpretation", "에이전트 해석"),
    }
}

const fn document_kind_label(kind: DocumentKind, locale: FixedLocale) -> &'static str {
    match kind {
        DocumentKind::ProjectArchitectureGuide => fixed(
            locale,
            "Project & Architecture Guide",
            "프로젝트 및 아키텍처 가이드",
        ),
        DocumentKind::DecisionReport => fixed(locale, "Decision Report", "결정 보고서"),
        DocumentKind::ImplementationPlan => fixed(locale, "Implementation Plan", "구현 계획"),
        DocumentKind::HandoffResume => fixed(locale, "Handoff / Resume", "인계 / 재개"),
    }
}

fn decision_choice_label(choice: &DecisionChoice, locale: FixedLocale) -> String {
    match choice {
        DecisionChoice::Alternative { alternative_key } => format!(
            "{}: {}",
            fixed(locale, "alternative", "대안"),
            alternative_key
        ),
        DecisionChoice::Delegation { delegate_to } => format!(
            "{}: {}",
            fixed(locale, "delegated to", "위임 대상"),
            delegate_to
        ),
    }
}

const fn brief_decision_state_label(
    state: BriefDecisionState,
    locale: FixedLocale,
) -> &'static str {
    match state {
        BriefDecisionState::Current => fixed(locale, "current", "현재 적용"),
        BriefDecisionState::StaleBasis => fixed(locale, "stale basis", "오래된 근거"),
        BriefDecisionState::ReviewRequired => fixed(locale, "review required", "검토 필요"),
        BriefDecisionState::Superseded => fixed(locale, "superseded", "대체됨"),
        BriefDecisionState::UnavailableBasis => {
            fixed(locale, "basis unavailable", "근거 사용 불가")
        }
    }
}

const fn source_freshness_label(state: SourceFreshness, locale: FixedLocale) -> &'static str {
    match state {
        SourceFreshness::Current => fixed(locale, "current", "현재"),
        SourceFreshness::Stale => fixed(locale, "stale", "오래됨"),
        SourceFreshness::Unavailable => fixed(locale, "unavailable", "사용 불가"),
        SourceFreshness::Unknown => fixed(locale, "unknown", "알 수 없음"),
    }
}

const fn capability_label(capability: Capability, locale: FixedLocale) -> &'static str {
    match capability {
        Capability::Inventory => fixed(locale, "inventory", "목록"),
        Capability::AgentAssisted => fixed(locale, "agent-assisted", "에이전트 보조"),
        Capability::Structural => fixed(locale, "structural", "구조 분석"),
        Capability::Semantic => fixed(locale, "semantic", "의미 분석"),
        Capability::Ecosystem => fixed(locale, "ecosystem", "생태계 분석"),
    }
}

const fn capability_key(capability: Capability) -> &'static str {
    match capability {
        Capability::Inventory => "inventory",
        Capability::AgentAssisted => "agent_assisted",
        Capability::Structural => "structural",
        Capability::Semantic => "semantic",
        Capability::Ecosystem => "ecosystem",
    }
}

const fn capability_state_label(state: CapabilityState, locale: FixedLocale) -> &'static str {
    match state {
        CapabilityState::Available => fixed(locale, "available", "사용 가능"),
        CapabilityState::Unavailable => fixed(locale, "unavailable", "사용 불가"),
        CapabilityState::Unsupported => fixed(locale, "unsupported", "지원하지 않음"),
        CapabilityState::Partial => fixed(locale, "partial", "일부 가능"),
        CapabilityState::Failed => fixed(locale, "failed", "실패"),
        CapabilityState::Stale => fixed(locale, "stale", "오래됨"),
    }
}

fn optional_language_label(language: Option<&Language>, locale: FixedLocale) -> String {
    language.map_or_else(
        || fixed(locale, "all languages", "모든 언어").to_owned(),
        |value| language_label(value, locale),
    )
}

fn language_label(language: &Language, locale: FixedLocale) -> String {
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
        Language::Shell => fixed(locale, "Shell", "셸").to_owned(),
        Language::Go => "Go".to_owned(),
        Language::OtherText(name) => name.clone(),
        Language::UnknownText => fixed(locale, "unknown text", "알 수 없는 텍스트").to_owned(),
    }
}

fn language_key(language: &Language) -> String {
    match language {
        Language::Java => "java".to_owned(),
        Language::Python => "python".to_owned(),
        Language::JavaScript => "javascript".to_owned(),
        Language::TypeScript => "typescript".to_owned(),
        Language::C => "c".to_owned(),
        Language::Cpp => "cpp".to_owned(),
        Language::Rust => "rust".to_owned(),
        Language::Markdown => "markdown".to_owned(),
        Language::Json => "json".to_owned(),
        Language::Yaml => "yaml".to_owned(),
        Language::Toml => "toml".to_owned(),
        Language::Xml => "xml".to_owned(),
        Language::Shell => "shell".to_owned(),
        Language::Go => "go".to_owned(),
        Language::OtherText(value) => format!("other:{value}"),
        Language::UnknownText => "unknown_text".to_owned(),
    }
}

fn code_entity_kind_label(kind: &CodeEntityKind, locale: FixedLocale) -> String {
    match kind {
        CodeEntityKind::Repository => fixed(locale, "repository", "저장소").to_owned(),
        CodeEntityKind::Package => fixed(locale, "package", "패키지").to_owned(),
        CodeEntityKind::Module => fixed(locale, "module", "모듈").to_owned(),
        CodeEntityKind::Namespace => fixed(locale, "namespace", "네임스페이스").to_owned(),
        CodeEntityKind::File => fixed(locale, "file", "파일").to_owned(),
        CodeEntityKind::Class => fixed(locale, "class", "클래스").to_owned(),
        CodeEntityKind::Interface => fixed(locale, "interface", "인터페이스").to_owned(),
        CodeEntityKind::Trait => "trait".to_owned(),
        CodeEntityKind::Struct => "struct".to_owned(),
        CodeEntityKind::Enum => "enum".to_owned(),
        CodeEntityKind::Type => fixed(locale, "type", "타입").to_owned(),
        CodeEntityKind::Function => fixed(locale, "function", "함수").to_owned(),
        CodeEntityKind::Method => fixed(locale, "method", "메서드").to_owned(),
        CodeEntityKind::Field => fixed(locale, "field", "필드").to_owned(),
        CodeEntityKind::Test => fixed(locale, "test", "테스트").to_owned(),
        CodeEntityKind::Configuration => fixed(locale, "configuration", "설정").to_owned(),
        CodeEntityKind::Document => fixed(locale, "document", "문서").to_owned(),
        CodeEntityKind::LanguageSpecific(name) => name.clone(),
    }
}

const fn context_role_label(role: ContextItemRole, locale: FixedLocale) -> &'static str {
    match role {
        ContextItemRole::Goal => fixed(locale, "goal", "목표"),
        ContextItemRole::Fact => fixed(locale, "fact", "사실"),
        ContextItemRole::Assumption => fixed(locale, "assumption", "가정"),
        ContextItemRole::Constraint => fixed(locale, "constraint", "제약"),
        ContextItemRole::Preference => fixed(locale, "preference", "선호"),
        ContextItemRole::Risk => fixed(locale, "risk", "위험"),
        ContextItemRole::Learning => fixed(locale, "learning", "학습"),
        ContextItemRole::KnownLimit => fixed(locale, "known limit", "알려진 한계"),
    }
}

const fn work_state_label(state: WorkState, locale: FixedLocale) -> &'static str {
    match state {
        WorkState::InProgress => fixed(locale, "in progress", "진행 중"),
        WorkState::Paused => fixed(locale, "paused", "일시 중지"),
        WorkState::Completed => fixed(locale, "completed", "완료"),
        WorkState::Abandoned => fixed(locale, "abandoned", "중단"),
        WorkState::Superseded => fixed(locale, "superseded", "대체됨"),
    }
}

const fn verification_state_label(state: VerificationState, locale: FixedLocale) -> &'static str {
    match state {
        VerificationState::NotRun => fixed(locale, "not run", "실행하지 않음"),
        VerificationState::Partial => fixed(locale, "partial", "일부 검증"),
        VerificationState::Passed => fixed(locale, "passed", "통과"),
        VerificationState::Failed => fixed(locale, "failed", "실패"),
    }
}

fn verification_fact_label(
    fact: &volicord_context::VerificationFact,
    locale: FixedLocale,
) -> String {
    let mut value = verification_state_label(fact.state, locale).to_owned();
    if let Some(outcome) = &fact.outcome {
        value.push_str(": ");
        value.push_str(outcome);
    }
    if let Some(source) = fact.source_id {
        value.push_str(&format!(" (Source {source})"));
    }
    value
}

const fn user_review_label(state: UserReviewState, locale: FixedLocale) -> &'static str {
    match state {
        UserReviewState::NotRequested => fixed(locale, "not requested", "요청하지 않음"),
        UserReviewState::Pending => fixed(locale, "pending", "대기 중"),
        UserReviewState::Reviewed => fixed(locale, "reviewed", "검토됨"),
    }
}

const fn user_acceptance_label(state: UserAcceptanceState, locale: FixedLocale) -> &'static str {
    match state {
        UserAcceptanceState::NotRequested => fixed(locale, "not requested", "요청하지 않음"),
        UserAcceptanceState::Pending => fixed(locale, "pending", "대기 중"),
        UserAcceptanceState::Accepted => fixed(locale, "accepted", "수락됨"),
        UserAcceptanceState::Rejected => fixed(locale, "rejected", "거부됨"),
    }
}

const fn projection_issue_kind_label(
    kind: crate::ProjectionIssueKind,
    locale: FixedLocale,
) -> &'static str {
    match kind {
        crate::ProjectionIssueKind::Bound => fixed(locale, "bounded omission", "범위 제한 생략"),
        crate::ProjectionIssueKind::WrongProject => fixed(locale, "wrong Project", "다른 프로젝트"),
        crate::ProjectionIssueKind::PartialCapability => {
            fixed(locale, "partial capability", "일부 기능")
        }
        crate::ProjectionIssueKind::UnavailableCapability => {
            fixed(locale, "unavailable capability", "사용 불가 기능")
        }
        crate::ProjectionIssueKind::UnsupportedCapability => {
            fixed(locale, "unsupported capability", "미지원 기능")
        }
        crate::ProjectionIssueKind::FailedCapability => {
            fixed(locale, "failed capability", "실패한 기능")
        }
        crate::ProjectionIssueKind::StaleCapability => {
            fixed(locale, "stale capability", "오래된 기능")
        }
        crate::ProjectionIssueKind::SourceUnavailable => {
            fixed(locale, "Source unavailable", "Source 사용 불가")
        }
        crate::ProjectionIssueKind::SourceStale => fixed(locale, "Source stale", "Source 오래됨"),
        crate::ProjectionIssueKind::CandidateInspection => {
            fixed(locale, "Candidate inspection", "후보 검사")
        }
        crate::ProjectionIssueKind::CandidateUnavailable => fixed(
            locale,
            "Candidate data unavailable",
            "후보 데이터 사용 불가",
        ),
        crate::ProjectionIssueKind::CandidateUnsupported => {
            fixed(locale, "Candidate data unsupported", "후보 데이터 미지원")
        }
        crate::ProjectionIssueKind::CandidateCorrupt => {
            fixed(locale, "Candidate data corrupt", "후보 데이터 손상")
        }
        crate::ProjectionIssueKind::CandidateRepairRequired => {
            fixed(locale, "Candidate repair required", "후보 복구 필요")
        }
        crate::ProjectionIssueKind::CandidateFailed => {
            fixed(locale, "Candidate read failed", "후보 읽기 실패")
        }
    }
}

fn format_scope(paths: &[String], components: &[String]) -> String {
    format!(
        "paths=[{}]; components=[{}]",
        paths.join(", "),
        components.join(", ")
    )
}

fn display_strings(values: &[String], locale: FixedLocale) -> String {
    if values.is_empty() {
        fixed(locale, "none", "없음").to_owned()
    } else {
        values.join(", ")
    }
}

const fn yes_no(value: bool, locale: FixedLocale) -> &'static str {
    if value {
        fixed(locale, "yes", "예")
    } else {
        fixed(locale, "no", "아니요")
    }
}

const fn fixed(locale: FixedLocale, english: &'static str, korean: &'static str) -> &'static str {
    match locale {
        FixedLocale::English => english,
        FixedLocale::Korean => korean,
    }
}

fn escape_markdown(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
