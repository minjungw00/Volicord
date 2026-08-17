use crate::{
    BriefDecisionState, CapabilityGap, MapRelationClass, ProjectProjection, ProjectionIssue,
};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error as StdError;
use std::fmt;
use volicord_context::{DecisionId, ProjectId, SourceFreshness, SourceId, TimestampMicros};
use volicord_repository_intelligence::{
    AnalysisSnapshotId, CapabilityReport, RepositorySnapshotId,
};

pub const GENERATED_DOCUMENT_FORMAT_KIND: &str = "volicord.generated_document";
pub const GENERATED_DOCUMENT_METADATA_VERSION: u32 = 2;

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
    /// Arbitrary BCP-47-like or user-provided language label. It is recorded
    /// and never checked against an allowlist.
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

fn build_body(
    kind: DocumentKind,
    projection: &ProjectProjection,
    locale: FixedLocale,
) -> DocumentBody {
    match kind {
        DocumentKind::ProjectArchitectureGuide => architecture_body(projection, locale),
        DocumentKind::DecisionReport => decision_body(projection, locale),
        DocumentKind::ImplementationPlan => implementation_body(projection, locale),
        DocumentKind::HandoffResume => handoff_body(projection, locale),
    }
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
                "{} ({:?}, {:?})",
                entity.display_name, entity.kind, entity.language
            ),
            source_basis: vec![entity.source_id],
            decision_basis: Vec::new(),
            analysis_basis: vec![entity.analysis_snapshot],
            explicit_inference: false,
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
                "{:?}: {:?}; rationale={}; applicability={:?}",
                decision.state,
                decision.choice,
                decision.user_rationale.as_deref().unwrap_or("not recorded"),
                projection
                    .decision_context_code
                    .iter()
                    .find(|link| link.decision_id == decision.decision_id)
                    .map(|link| (&link.declared_paths, &link.declared_components))
            ),
            source_basis: decision.source_basis.clone(),
            decision_basis: vec![decision.decision_id],
            analysis_basis: Vec::new(),
            explicit_inference: false,
            uncertainty: decision
                .uncertainty_and_limits
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
                "architecture",
                fixed(
                    locale,
                    "Repository architecture evidence",
                    "저장소 아키텍처 근거",
                ),
                architecture_claims,
            ),
            section(
                "decisions",
                fixed(locale, "Architecture decisions", "아키텍처 결정"),
                decision_claims,
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
                    "state={:?}; choice={:?}; user rationale={}; agent recommendation={}; assumptions={:?}; revisit triggers={:?}; scope={:?}; code links={:?}",
                    decision.state,
                    decision.choice,
                    decision.user_rationale.as_deref().unwrap_or("not recorded"),
                    decision.recommendation_rationale,
                    decision.assumptions,
                    decision.revisit_triggers,
                    link.map(|value| (
                        &value.declared_paths,
                        &value.declared_components,
                        &value.declared_work_contexts
                    )),
                    link.map(|value| &value.related_code_entities),
                ),
                source_basis: decision.source_basis.clone(),
                decision_basis: vec![decision.decision_id],
                analysis_basis: Vec::new(),
                explicit_inference: false,
                uncertainty: decision
                    .uncertainty_and_limits
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
            section(
                "decisions",
                fixed(locale, "Decision trail", "결정 이력"),
                claims,
            ),
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
                "Resolve '{}' (frontier={}, unlocks={:?}, blocked={:?})",
                question.prompt,
                question.on_current_frontier,
                question.what_the_answer_unlocks,
                question.blocked_basis
            ),
            source_basis: question.source_basis.clone(),
            decision_basis: Vec::new(),
            analysis_basis: Vec::new(),
            explicit_inference: true,
            uncertainty: question.blocked_basis.clone(),
        });
    }
    if let Some(checkpoint) = projection.resume.latest_meaningful_checkpoint.as_ref() {
        plan.push(GeneratedDocumentClaim {
            identity: format!("checkpoint-next:{}", checkpoint.id),
            class: ClaimClass::CanonicalContext,
            text: format!(
                "Next step: {}; affected paths={:?}; verification={:?}; known limits={:?}",
                checkpoint.next_step,
                checkpoint.changed_paths,
                checkpoint.verification,
                checkpoint.known_limits
            ),
            source_basis: checkpoint.source_basis.clone(),
            decision_basis: checkpoint.applied_decisions.clone(),
            analysis_basis: Vec::new(),
            explicit_inference: false,
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
                text: format!("{:?}: {}", item.role, item.statement),
                source_basis: item.source_basis.clone(),
                decision_basis: Vec::new(),
                analysis_basis: Vec::new(),
                explicit_inference: false,
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
                "{} (frontier={}, blocked={:?}, unlocks={:?})",
                question.prompt,
                question.on_current_frontier,
                question.blocked_basis,
                question.what_the_answer_unlocks
            ),
            source_basis: question.source_basis.clone(),
            decision_basis: Vec::new(),
            analysis_basis: Vec::new(),
            explicit_inference: false,
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
                    uncertainty: projection.resume.known_limits.clone(),
                }],
            ),
            gap_section(projection, locale),
        ],
    }
}

fn timeline_section(projection: &ProjectProjection, locale: FixedLocale) -> DocumentSection {
    let claims = projection
        .checkpoint_timeline
        .iter()
        .map(|entry| GeneratedDocumentClaim {
            identity: format!("checkpoint:{}", entry.checkpoint.id),
            class: ClaimClass::CanonicalContext,
            text: format!(
                "goal={}; work={:?}; verification={:?}; user review={:?}; user acceptance={:?}; changes={:?}; next={}",
                entry.checkpoint.goal,
                entry.work_state,
                entry.verification,
                entry.user_review,
                entry.user_acceptance,
                entry.checkpoint.changed_paths,
                entry.checkpoint.next_step,
            ),
            source_basis: entry.checkpoint.source_basis.clone(),
            decision_basis: entry.checkpoint.applied_decisions.clone(),
            analysis_basis: Vec::new(),
            explicit_inference: false,
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
                "gap:{}:{:?}:{:?}:{}",
                gap.analysis_snapshot, gap.capability, gap.language, gap.area
            ),
            class: ClaimClass::RepositoryObservation,
            text: format!(
                "state={:?}; capability={:?}; language={:?}; area={}; reason={}; affected={:?}; usable remainder={:?}",
                gap.state,
                gap.capability,
                gap.language,
                gap.area,
                gap.reason,
                gap.affected_areas,
                gap.usable_remainder
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
            uncertainty: vec![gap.reason.clone()],
        })
        .collect::<Vec<_>>();
    claims.extend(projection.issues.iter().map(|issue| {
        GeneratedDocumentClaim {
            identity: format!("omission:{}:{}", issue.affected_scope, issue.identity),
            class: ClaimClass::RepositoryObservation,
            text: format!(
                "{}: {} ({:?}; omitted_count={})",
                issue.affected_scope, issue.reason, issue.kind, issue.omitted_count
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
            uncertainty: vec![issue.reason.clone()],
        }
    }));
    claims.sort_by(|left, right| left.identity.cmp(&right.identity));
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
        .collect::<Vec<_>>();
    repository_snapshots.sort();
    repository_snapshots.dedup();
    let mut analysis_snapshots = body
        .sections
        .iter()
        .flat_map(|section| &section.claims)
        .flat_map(|claim| claim.analysis_basis.iter().copied())
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
        .map(|destination| format!("{:?}:{}", destination.output_format, destination.path))
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
    output.push_str(&escape_markdown(&body.title));
    output.push_str("\n\n");
    render_metadata_markdown(&mut output, metadata, locale);
    for section in &body.sections {
        output.push_str("## ");
        output.push_str(&escape_markdown(&section.title));
        output.push_str("\n\n");
        if section.claims.is_empty() {
            output.push_str(fixed(locale, "No grounded items.", "grounded 항목 없음."));
            output.push_str("\n\n");
        }
        for claim in &section.claims {
            output.push_str("- **[");
            output.push_str(claim_class_label(claim.class));
            output.push_str("]** ");
            if claim.explicit_inference {
                output.push_str("**[Inference]** ");
            }
            output.push_str(&escape_markdown(&claim.text));
            output.push_str("  \n  ");
            output.push_str(&escape_markdown(&claim_basis(claim)));
            if !claim.uncertainty.is_empty() {
                output.push_str("  \n  uncertainty: ");
                output.push_str(&escape_markdown(&claim.uncertainty.join("; ")));
            }
            output.push('\n');
        }
        output.push('\n');
    }
    output
}

fn render_html(metadata: &DocumentMetadata, body: &DocumentBody, locale: FixedLocale) -> String {
    let mut output = String::from("<!doctype html><html lang=\"");
    output.push_str(&escape_html(&metadata.requested_language));
    output.push_str(
        "\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><style>body{font-family:system-ui,sans-serif;max-width:72rem;margin:2rem auto;padding:0 1rem;line-height:1.5}dl{display:grid;grid-template-columns:max-content 1fr;gap:.25rem 1rem}dt{font-weight:700}.claim{border-left:.3rem solid #667085;padding:.6rem 1rem;margin:.75rem 0;background:#f8fafc}.basis,.uncertainty{color:#475467;font-size:.92rem}.inference{font-weight:700;color:#9a3412}code{overflow-wrap:anywhere}</style><title>",
    );
    output.push_str(&escape_html(&body.title));
    output.push_str("</title></head><body><main><h1>");
    output.push_str(&escape_html(&body.title));
    output.push_str("</h1>");
    render_metadata_html(&mut output, metadata, locale);
    for section in &body.sections {
        output.push_str("<section data-section=\"");
        output.push_str(&escape_html(&section.key));
        output.push_str("\"><h2>");
        output.push_str(&escape_html(&section.title));
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
            output.push_str("<article class=\"claim\" data-claim-id=\"");
            output.push_str(&escape_html(&claim.identity));
            output.push_str("\"><strong>[");
            output.push_str(claim_class_label(claim.class));
            output.push_str("]</strong> ");
            if claim.explicit_inference {
                output.push_str("<span class=\"inference\">[Inference]</span> ");
            }
            output.push_str(&escape_html(&claim.text));
            output.push_str("<div class=\"basis\">");
            output.push_str(&escape_html(&claim_basis(claim)));
            output.push_str("</div>");
            if !claim.uncertainty.is_empty() {
                output.push_str("<div class=\"uncertainty\">uncertainty: ");
                output.push_str(&escape_html(&claim.uncertainty.join("; ")));
                output.push_str("</div>");
            }
            output.push_str("</article>");
        }
        output.push_str("</section>");
    }
    output.push_str("</main></body></html>\n");
    output
}

fn render_metadata_markdown(output: &mut String, metadata: &DocumentMetadata, locale: FixedLocale) {
    output.push_str("## ");
    output.push_str(fixed(locale, "Grounding metadata", "Grounding 메타데이터"));
    output.push_str("\n\n");
    for (label, value) in metadata_pairs(metadata) {
        output.push_str("- **");
        output.push_str(label);
        output.push_str(":** ");
        output.push_str(&escape_markdown(&value));
        output.push('\n');
    }
    output.push('\n');
}

fn render_metadata_html(output: &mut String, metadata: &DocumentMetadata, locale: FixedLocale) {
    output.push_str("<section data-section=\"metadata\"><h2>");
    output.push_str(&escape_html(fixed(
        locale,
        "Grounding metadata",
        "Grounding 메타데이터",
    )));
    output.push_str("</h2><dl>");
    for (label, value) in metadata_pairs(metadata) {
        output.push_str("<dt>");
        output.push_str(&escape_html(label));
        output.push_str("</dt><dd>");
        output.push_str(&escape_html(&value));
        output.push_str("</dd>");
    }
    output.push_str("</dl></section>");
}

fn metadata_pairs(metadata: &DocumentMetadata) -> Vec<(&'static str, String)> {
    vec![
        (
            "format",
            format!("{}@{}", metadata.format_kind, metadata.format_version),
        ),
        ("document", format!("{:?}", metadata.document_kind)),
        ("project", metadata.project_id.to_string()),
        (
            "canonical revision",
            metadata.canonical_revision.to_string(),
        ),
        (
            "generated at",
            metadata.generated_at.as_unix_micros().to_string(),
        ),
        (
            "generator",
            format!(
                "{}; agent={:?}; model={:?}",
                metadata.generator.generator, metadata.generator.agent, metadata.generator.model
            ),
        ),
        ("requested language", metadata.requested_language.clone()),
        (
            "repository snapshots",
            join_display(&metadata.repository_snapshots),
        ),
        (
            "analysis snapshots",
            join_display(&metadata.analysis_snapshots),
        ),
        (
            "included Decisions",
            metadata
                .included_decisions
                .iter()
                .map(|decision| {
                    format!(
                        "{}@{}:{:?}",
                        decision.decision_id, decision.revision, decision.state
                    )
                })
                .collect::<Vec<_>>()
                .join(", "),
        ),
        (
            "used Sources",
            metadata
                .used_sources
                .iter()
                .map(|source| format!("{}:{:?}", source.source_id, source.freshness))
                .collect::<Vec<_>>()
                .join(", "),
        ),
        (
            "capability coverage",
            metadata
                .capability_coverage
                .iter()
                .map(|report| {
                    format!(
                        "{:?}/{:?}/{}={:?} files:{} entities:{} relations:{}",
                        report.language,
                        report.capability,
                        report.area.path,
                        report.state,
                        report.coverage.covered_file_count,
                        report.coverage.covered_entity_count,
                        report.coverage.covered_relation_count
                    )
                })
                .collect::<Vec<_>>()
                .join("; "),
        ),
        (
            "known gaps",
            metadata
                .capability_gaps
                .iter()
                .map(|gap| {
                    format!(
                        "{:?}/{:?}/{}: {}",
                        gap.language, gap.capability, gap.area, gap.reason
                    )
                })
                .collect::<Vec<_>>()
                .join("; "),
        ),
        (
            "omissions",
            metadata
                .omissions
                .iter()
                .map(|issue| {
                    format!(
                        "{}:{}:omitted_count={}",
                        issue.affected_scope, issue.reason, issue.omitted_count
                    )
                })
                .collect::<Vec<_>>()
                .join("; "),
        ),
        (
            "requested destinations",
            metadata.requested_destination_basis.join(", "),
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

const fn claim_class_label(class: ClaimClass) -> &'static str {
    match class {
        ClaimClass::CanonicalContext => "Canonical Context",
        ClaimClass::RepositoryObservation => "Repository Observation",
        ClaimClass::StructuralFact => "Structural Fact",
        ClaimClass::SemanticResult => "Semantic Result",
        ClaimClass::AgentInterpretation => "Agent Interpretation",
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
