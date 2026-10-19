//! One question-scoped ordinary-reading contract. Quotations and raw history
//! stay identified as quotations; generated prose never replaces recorded actions.
use crate::*;
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExplanationProvenance {
    pub project_id: String,
    pub subject: ExplanationSubject,
    pub question: String,
    pub language: String,
    pub fingerprint: String,
    /// Binds the exact output including edited wording and claim bindings.
    pub realization_fingerprint: String,
    pub generated_at_unix_micros: i64,
    pub generator: ExplanationGenerator,
    pub generator_identity_status: String,
    pub semantic_approval_status: String,
    pub citations: Vec<ExplanationCitation>,
    pub evidence: Vec<ExplanationEvidence>,
    /// Offered preparation evidence that the realization did not cite. The
    /// retained explanation preserves it; ordinary provenance prioritizes use.
    pub uncited_evidence_count: usize,
    pub source_status: Vec<serde_json::Value>,
    pub conflicts: Vec<serde_json::Value>,
}
impl From<&RetainedExplanation> for ExplanationProvenance {
    fn from(e: &RetainedExplanation) -> Self {
        let cited: std::collections::BTreeSet<_> = e
            .realization
            .generated_units()
            .flat_map(|p| p.evidence_keys.iter())
            .collect();
        let evidence: Vec<_> = e
            .evidence
            .iter()
            .filter(|item| cited.contains(&item.key))
            .cloned()
            .collect();
        let uncited_evidence_count = e.evidence.len() - evidence.len();
        Self {
            project_id: e.project_id.clone(),
            subject: e.subject,
            question: e.question.clone(),
            language: e.realization.language.clone(),
            fingerprint: e.realization.plan_fingerprint.clone(),
            realization_fingerprint: explanation_text_hash(
                &serde_json::to_string(&e.realization).unwrap_or_default(),
            ),
            generated_at_unix_micros: e.generated_at_unix_micros,
            generator: e.realization.generator.clone(),
            generator_identity_status: e.generator_identity_status.clone(),
            semantic_approval_status: e.semantic_approval_status.clone(),
            citations: e
                .realization
                .generated_units()
                .flat_map(|p| p.citations.clone())
                .collect(),
            evidence,
            uncited_evidence_count,
            source_status: e.source_status.clone(),
            conflicts: e.conflicts.clone(),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerRole {
    GeneratedInterpretation,
    DeterministicFacts,
    Unavailable,
}
/// Canonical continuation basis, independent of generated provenance. Source
/// status describes support for the recorded direction, not current code truth.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RecordedActionBasis {
    pub work_item_id: String,
    pub checkpoint_id: String,
    pub revision: u64,
    pub field: String,
    pub recorded_text: String,
    pub source_ids: Vec<String>,
    pub source_status: Vec<serde_json::Value>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct QuestionAnswer {
    pub question: String,
    pub text: String,
    pub role: AnswerRole,
    pub evidence_keys: Vec<String>,
    pub citations: Vec<ExplanationCitation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_action: Option<RecordedActionBasis>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReaderEvidenceEntry {
    pub unit_index: usize,
    pub evidence_keys: Vec<String>,
    pub citations: Vec<ExplanationCitation>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingCautionKind {
    FailedCheck,
    PartialCheck,
    NotRun,
    CoverageUnknown,
    HistoricalCoverage,
    SourceBodyUnavailable,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReadingCaution {
    pub kind: ReadingCautionKind,
    pub checkpoint: Option<VerificationReference>,
    pub source_id: Option<String>,
    pub historical: bool,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct QuestionAnswers {
    #[serde(skip)]
    pub(crate) fixed_locale: FixedLocale,
    /// Independently admitted card publications. Never merged into article prose
    /// or represented by the article's provenance/currentness.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_name: Option<Box<QuestionAnswers>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_summary: Option<Box<QuestionAnswers>>,
    pub evidence_index: Vec<ReaderEvidenceEntry>,
    pub cautions: Vec<ReadingCaution>,
    pub prose: Vec<QuestionAnswer>,
    pub facts: Vec<QuestionAnswer>,
    pub explanation_state: ExplanationState,
    pub diagnostic: Option<String>,
    pub provenance: Option<ExplanationProvenance>,
    pub source_excerpts: Vec<ExplanationSourceExcerpt>,
    pub omitted_source_excerpt_count: usize,
}
impl QuestionAnswers {
    /// Authored derived heading from the permitted output, with original Goal fallback
    /// owned by the caller. This is interpretation, never a renamed canonical Work.
    pub fn reading_title(&self) -> Option<&str> {
        self.prose
            .iter()
            .find(|a| a.question == "Heading" && a.role == AnswerRole::GeneratedInterpretation)
            .map(|a| a.text.as_str())
    }
    pub fn synopsis(&self) -> Option<&QuestionAnswer> {
        self.prose
            .iter()
            .find(|a| a.question == "Synopsis" && a.role == AnswerRole::GeneratedInterpretation)
    }
    pub fn card_name(&self) -> Option<&str> {
        self.work_name
            .as_ref()
            .and_then(|name| name.reading_title())
    }
    pub fn card_summary(&self) -> Option<&QuestionAnswer> {
        self.work_summary
            .as_ref()
            .and_then(|summary| summary.synopsis())
    }
    pub fn publication_provenances(&self) -> Vec<ExplanationProvenance> {
        self.work_name
            .iter()
            .chain(self.work_summary.iter())
            .filter_map(|scope| scope.provenance.clone())
            .chain(self.provenance.clone())
            .collect()
    }
    /// The sole task-direction selection rule. Generated NextStep prose is an
    /// interpretation alongside this fact; explanation repair is never an action.
    pub fn next_step_answer(&self) -> Option<&QuestionAnswer> {
        self.facts.iter().find(|a| {
            matches!(
                a.question.as_str(),
                "RecordedNextStep" | "NextStepAvailability"
            )
        })
    }
    pub fn recorded_next_action(&self) -> Option<&RecordedActionBasis> {
        self.next_step_answer()
            .and_then(|a| a.recorded_action.as_ref())
    }
    pub fn text(&self) -> String {
        let article = self
            .prose
            .iter()
            .chain(&self.facts)
            .map(|a| {
                let references = a
                    .citations
                    .iter()
                    .map(|c| {
                        format!(
                            "{}:{}–{} ({:?})",
                            c.locator, c.start_line, c.end_line, c.use_kind
                        )
                    })
                    .collect::<Vec<_>>();
                if references.is_empty() {
                    a.text.clone()
                } else {
                    format!("{} [{}]", a.text, references.join("; "))
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        if self.work_name.is_none() && self.work_summary.is_none() {
            return article;
        }
        let mut scopes = Vec::new();
        if let Some(name) = &self.work_name {
            scopes.push(format!(
                "{}: {}",
                fixed(self.fixed_locale, "Work name", "작업 이름"),
                name.text()
            ));
        }
        if let Some(summary) = &self.work_summary {
            scopes.push(format!(
                "{}: {}",
                fixed(self.fixed_locale, "Work summary", "작업 요약"),
                summary.text()
            ));
        }
        scopes.push(format!(
            "{}: {article}",
            fixed(
                self.fixed_locale,
                "Article and recorded facts",
                "상세 설명과 기록된 사실"
            )
        ));
        scopes.join("\n\n")
    }
}
fn fixed(locale: FixedLocale, en: &str, ko: &str) -> String {
    match locale {
        FixedLocale::English => en,
        FixedLocale::Korean => ko,
    }
    .into()
}

pub fn explanation_answers(
    readings: &[ExplanationReading],
    language: &str,
    locale: FixedLocale,
) -> QuestionAnswers {
    let reading = readings
        .iter()
        .find(|e| e.language == language)
        .or_else(|| readings.iter().find(|e| e.language == "*"));
    let state = reading.map_or(ExplanationState::Missing, |e| e.state);
    let current = reading
        .filter(|e| e.state == ExplanationState::Current)
        .and_then(|e| e.content.as_ref());
    let prose: Vec<QuestionAnswer> = if let Some(e) = current {
        e.realization
            .paragraphs
            .iter()
            .map(|p| QuestionAnswer {
                question: format!("{:?}", p.question),
                text: p.text.clone(),
                role: AnswerRole::GeneratedInterpretation,
                evidence_keys: p.evidence_keys.clone(),
                citations: p.citations.clone(),
                recorded_action: None,
            })
            .collect()
    } else {
        let (en,ko)=match state {
            ExplanationState::Stale=>("Explanation is stale; prepare and generate again.","설명의 근거가 변경되었습니다. 다시 준비하고 생성하세요."),
            ExplanationState::Corrupt=>("Explanation is corrupt; delete and regenerate.","설명이 손상되었습니다. 삭제한 뒤 다시 생성하세요."),
            ExplanationState::Unsupported=>("Explanation format is unsupported; delete and regenerate.","설명 형식을 지원하지 않습니다. 삭제한 뒤 다시 생성하세요."),
            ExplanationState::Deleted=>("Explanation was explicitly deleted; canonical facts remain readable. Generation requires a new explicit request.","설명이 명시적으로 삭제되었습니다. Canonical 사실은 계속 읽을 수 있습니다. 생성하려면 새로운 명시적 요청이 필요합니다."),
            ExplanationState::Withheld=>("Explanation is withheld by the canonical forgetting read barrier; repair local cleanup before reading affected content.","정식 기록 삭제의 읽기 차단으로 설명이 보류되었습니다. 영향을 받은 내용을 읽기 전에 로컬 정리를 복구하세요."),
            _ if reading.is_some()=>("Explanation is unavailable; canonical facts remain readable. Inspect the diagnostic for the affected dependency.","설명을 사용할 수 없습니다. Canonical 사실은 계속 읽을 수 있습니다. 영향을 받은 의존성은 진단에서 확인하세요."),
            _=>("Interpretation has not been generated in this language. Prepare evidence and ask the active host to explain it.","이 언어의 해석이 아직 생성되지 않았습니다. 근거를 준비하고 현재 호스트에 설명을 요청하세요."),
        };
        vec![QuestionAnswer {
            question: "ExplanationAvailability".into(),
            text: fixed(locale, en, ko),
            role: AnswerRole::Unavailable,
            evidence_keys: Vec::new(),
            citations: Vec::new(),
            recorded_action: None,
        }]
    };
    let mut result = QuestionAnswers {
        fixed_locale: locale,
        work_name: None,
        work_summary: None,
        evidence_index: if current.is_some() {
            prose
                .iter()
                .enumerate()
                .map(|(unit_index, p)| ReaderEvidenceEntry {
                    unit_index,
                    evidence_keys: p.evidence_keys.clone(),
                    citations: p.citations.clone(),
                })
                .collect()
        } else {
            Vec::new()
        },
        cautions: Vec::new(),
        prose,
        facts: Vec::new(),
        explanation_state: state,
        diagnostic: reading.and_then(|e| e.diagnostic.clone()),
        provenance: current.map(Into::into),
        source_excerpts: if current.is_some() {
            reading
                .map(|r| r.source_excerpts.clone())
                .unwrap_or_default()
        } else {
            Vec::new()
        },
        omitted_source_excerpt_count: if current.is_some() {
            reading.map_or(0, |r| r.omitted_source_excerpt_count)
        } else {
            0
        },
    };
    if let Some(e) = current.filter(|e| matches!(e.subject, ExplanationSubject::Repository(_))) {
        result.facts.extend(repository_scope_facts(e, locale));
    }
    if current.is_some_and(|e| !e.conflicts.is_empty()) {
        result.facts.push(fact("Conflicts",fixed(locale,"The evidence includes contradiction or supersession; this interpretation does not resolve it.","근거에 모순 또는 대체 관계가 있습니다. 이 해석은 해당 관계를 해결하지 않습니다."),Vec::new()));
    }
    result
}
// Describe exactly retained scope, not a guessed Product boundary or semantic
// completeness. Root/directory inventory entries cannot inflate file coverage.
fn common_file_area(paths: &[&str]) -> String {
    let Some(first) = paths.first() else {
        return ".".into();
    };
    let mut prefix = first
        .rsplit_once('/')
        .map_or("", |(parent, _)| parent)
        .split('/')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>();
    for path in paths.iter().skip(1) {
        let parent = path
            .rsplit_once('/')
            .map_or("", |(parent, _)| parent)
            .split('/')
            .collect::<Vec<_>>();
        let count = prefix
            .iter()
            .zip(parent)
            .take_while(|(a, b)| **a == *b)
            .count();
        prefix.truncate(count);
    }
    if prefix.is_empty() {
        ".".into()
    } else {
        prefix.join("/")
    }
}
fn repository_scope_facts(e: &RetainedExplanation, locale: FixedLocale) -> Vec<QuestionAnswer> {
    let Some(basis) = e
        .evidence
        .iter()
        .find(|item| item.key == "repository_basis")
    else {
        return Vec::new();
    };
    let area_paths = |key: &str| {
        basis.content["repository"][key]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|area| area["path"].as_str())
            .collect::<Vec<_>>()
    };
    let included = basis.content["repository"]["included_areas"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|area| area["kind"].as_str() == Some("file"))
        .filter_map(|area| area["path"].as_str())
        .collect::<Vec<_>>();
    let cited = e
        .realization
        .generated_units()
        .flat_map(|p| &p.citations)
        .map(|c| c.locator.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let inventory_area = common_file_area(&included);
    let cited_area = common_file_area(&cited);
    let excluded = area_paths("excluded_areas").len();
    let unavailable = area_paths("unavailable_areas").len();
    let key = vec!["repository_basis".into()];
    let scope = match locale {
        FixedLocale::English => format!("Cited Source area: {cited_area} ({} files). Observed inventory: {inventory_area} ({} files). Inventory inclusion does not establish explanation coverage.", cited.len(), included.len()),
        FixedLocale::Korean => format!("인용 Source 영역: {cited_area} ({}개 파일). 관찰한 목록 범위: {inventory_area} ({}개 파일). 목록에 포함되어도 모든 파일을 설명한 것은 아닙니다.", cited.len(), included.len()),
    };
    let scope = if included.is_empty() {
        fixed(locale, "No included inventory files are retained; repository-wide coverage is not established.", "포함된 목록 파일이 보존되지 않았습니다. 저장소 전체 범위를 확인할 수 없습니다.")
    } else {
        scope
    };
    let capabilities = basis.content["capabilities"]
        .as_array()
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let failed = capabilities
        .iter()
        .filter(|c| c["state"].as_str() == Some("failed"))
        .count();
    let limited = capabilities
        .iter()
        .filter(|c| c["state"].as_str() != Some("available"))
        .count();
    let noncurrent = capabilities
        .iter()
        .filter(|c| c["freshness"]["state"].as_str() != Some("current"))
        .count();
    let limits = match locale {
        FixedLocale::English => format!("Excluded boundaries: {excluded}; unavailable or unexamined boundaries: {unavailable}. Analysis limits: {limited} capability reports ({failed} failed; {noncurrent} stale or unknown). Complete semantic understanding is not established; cited code remains usable."),
        FixedLocale::Korean => format!("제외 경계 {excluded}개, 이용 불가 또는 미조사 경계 {unavailable}개. 분석 제한 보고 {limited}건 (실패 {failed}건, 오래되었거나 최신성 미확인 {noncurrent}건). 완전한 의미 분석을 입증하지 않습니다. 인용 코드는 이 한계 안에서 읽을 수 있습니다."),
    };
    vec![
        fact("RepositoryScope", scope, key.clone()),
        fact("RepositoryCoverage", limits, key),
    ]
}

fn fact(question: &str, text: String, evidence_keys: Vec<String>) -> QuestionAnswer {
    QuestionAnswer {
        question: question.into(),
        text,
        role: AnswerRole::DeterministicFacts,
        evidence_keys,
        citations: Vec::new(),
        recorded_action: None,
    }
}

fn source_gap<'a>(
    statuses: impl Iterator<Item = &'a ReadingSourceStatus>,
    locale: FixedLocale,
) -> Option<QuestionAnswer> {
    let sources = statuses
        .map(|s| (s.source_id, s))
        .collect::<std::collections::BTreeMap<_, _>>();
    let unavailable = sources
        .values()
        .filter(|s| s.availability != Some(volicord_context::Availability::Available))
        .count();
    let noncurrent = sources
        .values()
        .filter(|s| s.freshness != volicord_context::SourceFreshness::Current)
        .count();
    if unavailable == 0 && noncurrent == 0 {
        return None;
    }
    Some(fact("SourceEvidenceGap", match locale {
        FixedLocale::English => format!("Supporting sources unavailable or unknown: {unavailable}; stale or unknown freshness: {noncurrent}. Recorded reports do not establish current repository behavior."),
        FixedLocale::Korean => format!("지원 근거 이용 불가 또는 미확인: {unavailable}; 오래됐거나 최신 여부 미확인: {noncurrent}. 기록된 보고로 현재 저장소 동작을 확정하지 않습니다."),
    }, sources.values().filter(|s| s.availability != Some(volicord_context::Availability::Available) || s.freshness != volicord_context::SourceFreshness::Current).map(|s| format!("source:{}:availability,freshness", s.source_id)).collect()))
}

pub fn work_answers(
    work: &UnderstandingWork,
    language: &str,
    locale: FixedLocale,
) -> QuestionAnswers {
    let mut result = explanation_answers(&work.reading.explanations, language, locale);
    result.work_name = Some(Box::new(explanation_answers(
        &work.reading.name_explanations,
        language,
        locale,
    )));
    result.work_summary = Some(Box::new(explanation_answers(
        &work.reading.summary_explanations,
        language,
        locale,
    )));
    for check in &work.reading.verification_report.checks {
        let historical = work
            .reading
            .answers
            .verification
            .as_ref()
            .is_none_or(|s| s.checkpoint_id.to_string() != check.original.checkpoint_id);
        let kind = match check.reported_state {
            ReportedCheckState::Failed => ReadingCautionKind::FailedCheck,
            ReportedCheckState::Partial => ReadingCautionKind::PartialCheck,
            ReportedCheckState::NotRun => ReadingCautionKind::NotRun,
            ReportedCheckState::Passed => {
                if check.applicability == VerificationApplicability::HistoricalAfterRecordedChange {
                    ReadingCautionKind::HistoricalCoverage
                } else {
                    ReadingCautionKind::CoverageUnknown
                }
            }
        };
        result.cautions.push(ReadingCaution {
            kind,
            checkpoint: Some(check.original.clone()),
            source_id: check.source_id.clone(),
            historical,
        });
    }
    for execution in &work.reading.verification_report.executions {
        if execution.body_state != CommandBodyState::Available {
            result.cautions.push(ReadingCaution {
                kind: ReadingCautionKind::SourceBodyUnavailable,
                checkpoint: None,
                source_id: Some(execution.source_id.clone()),
                historical: true,
            });
        }
    }

    // A permitted command excerpt is an execution observation even before any
    // Checkpoint. Exit status says nothing about unreported test assertions.
    let mut executions = std::collections::BTreeSet::new();
    for excerpt in &result.source_excerpts {
        let Some(execution) = &excerpt.execution else {
            continue;
        };
        if !executions.insert(excerpt.citation.source_id.clone()) {
            continue;
        }
        result.facts.push(fact(
            "SourceExecutionObservation",
            format!("{}: {} · exit {} · termination {}. {}",
                fixed(locale, "Observed command", "관측한 명령"),
                excerpt.citation.locator, execution["exit_code"], execution["termination"],
                fixed(locale,
                    "Scoped Source observation; exit status alone establishes no test assertions, Checkpoint verification or user acceptance.",
                    "범위가 지정된 Source 관측입니다. 종료 코드만으로 테스트 assertion, Checkpoint 검증이나 사용자 수락을 확정하지 않습니다.")),
            vec![format!("source:{}@{}:execution", excerpt.citation.source_id, excerpt.citation.source_revision)],
        ));
    }
    let direction = &work.reading.next_step;
    if let (ReadingRecord::Checkpoint(checkpoint_id), Some(recorded)) = (
        &direction.basis.record,
        direction
            .original_text
            .as_deref()
            .filter(|s| !s.trim().is_empty()),
    ) {
        let mut answer = fact(
            "RecordedNextStep",
            format!(
                "{}: {recorded}",
                fixed(
                    locale,
                    "Recorded next action quotation (original language)",
                    "기록된 다음 행동 인용 (원문 언어)"
                )
            ),
            vec![format!(
                "checkpoint:{checkpoint_id}@{}:next_step",
                direction.basis.revision
            )],
        );
        answer.recorded_action = Some(RecordedActionBasis {
            work_item_id: work.work_item_id.to_string(),
            checkpoint_id: checkpoint_id.to_string(),
            revision: direction.basis.revision,
            field: direction.basis.field.clone(),
            recorded_text: recorded.into(),
            source_ids: direction
                .basis
                .source_basis
                .iter()
                .map(ToString::to_string)
                .collect(),
            source_status: direction
                .basis
                .source_status
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "source_id": s.source_id.to_string(),
                        "availability": s.availability.map(|a| format!("{a:?}").to_lowercase()),
                        "freshness": format!("{:?}", s.freshness).to_lowercase(),
                        "snapshot_basis": s.snapshot_basis,
                    })
                })
                .collect(),
        });
        result.facts.push(answer);
    } else {
        let mut answer = fact(
            "NextStepAvailability",
            fixed(
                locale,
                if work.reading.answers.latest_state.is_none() {
                    "No next action is recorded: this Work has no Checkpoint."
                } else {
                    "No next action is recorded in the latest Checkpoint."
                },
                if work.reading.answers.latest_state.is_none() {
                    "다음 행동이 기록되지 않았습니다. 이 작업에는 Checkpoint가 없습니다."
                } else {
                    "최신 Checkpoint에 다음 행동이 기록되지 않았습니다."
                },
            ),
            Vec::new(),
        );
        answer.role = AnswerRole::Unavailable;
        result.facts.push(answer);
    }
    if let Some(gap) = source_gap(work.reading.evidence_source_status.iter(), locale) {
        result.facts.push(gap);
    }
    if !work.changed_paths.is_empty() {
        result.facts.push(fact(
            "RecordedCodeScope",
            format!(
                "{}: {}",
                fixed(locale, "Recorded changed paths", "기록된 변경 경로"),
                work.changed_paths.join(", ")
            ),
            work.reading
                .states
                .iter()
                .map(|s| {
                    format!(
                        "checkpoint:{}@{}:changed_paths",
                        s.checkpoint_id, s.checkpoint_revision
                    )
                })
                .collect(),
        ));
    }
    if let Some(gap) = work.reading.code_gap {
        result.facts.push(fact(
            "CodeEvidenceGap",
            match locale {
                FixedLocale::English => gap.english(),
                FixedLocale::Korean => gap.korean(),
            }
            .into(),
            Vec::new(),
        ));
    }
    let answers = &work.reading.answers;
    if result.provenance.is_none() {
        result.facts.push(fact(
            "RecordedGoal",
            format!(
                "{}: {}",
                fixed(locale, "Recorded Goal quotation", "기록된 목표 인용"),
                work.reading.goal.original_text.as_deref().unwrap_or("")
            ),
            vec![format!(
                "context_item:{}@{}:statement",
                work.work_item_id, work.reading.goal.basis.revision
            )],
        ));
    }

    let key = |s: &WorkStateObservation, field: &str| {
        vec![format!(
            "checkpoint:{}@{}:{field}",
            s.checkpoint_id, s.checkpoint_revision
        )]
    };
    for (question, values, en, ko, field) in [
        (
            "RecordedLimits",
            &work.reading.known_limits,
            "Recorded limitations in the latest Checkpoint",
            "최신 Checkpoint에 기록된 한계",
            "known_limits",
        ),
        (
            "RecordedNonGoals",
            &work.reading.non_goals,
            "Recorded non-goals in the latest Checkpoint",
            "최신 Checkpoint에 기록된 비목표",
            "non_goals",
        ),
    ] {
        if !values.is_empty() {
            result.facts.push(fact(
                question,
                format!("{}: {}", fixed(locale, en, ko), values.join("; ")),
                answers
                    .latest_state
                    .as_ref()
                    .map_or_else(Vec::new, |s| key(s, field)),
            ));
        }
    }
    result.facts.push(fact(
        "WorkState",
        format!(
            "{}: {}",
            fixed(locale, "Work", "작업"),
            if answers.latest_state.is_none() {
                match locale {
                    FixedLocale::English => {
                        "Goal-only Work is open; no Checkpoint work state is recorded"
                    }
                    FixedLocale::Korean => {
                        "Goal만 있는 작업은 열림이며 Checkpoint 작업 상태가 기록되지 않았습니다"
                    }
                }
            } else {
                work_state_label_from_understanding(work.state, locale)
            }
        ),
        answers.latest_state.as_ref().map_or_else(
            || {
                vec![format!(
                    "context_item:{}@{}:role",
                    work.work_item_id, work.reading.goal.basis.revision
                )]
            },
            |s| key(s, "work_state"),
        ),
    ));
    if work.reading.verification_report.checks.is_empty() {
        result.facts.push(fact(
            "VerificationState",
            fixed(
                locale,
                "Checkpoint verification: No verification recorded in a Checkpoint",
                "Checkpoint 검증: Checkpoint에 기록된 검증 없음",
            ),
            Vec::new(),
        ));
    }
    for check in &work.reading.verification_report.checks {
        let keys = vec![format!(
            "checkpoint:{}@{}:verification[{}]",
            check.original.checkpoint_id,
            check.original.checkpoint_revision,
            check.original.verification_index
        )];
        result.facts.push(fact(
            "VerificationState",
            format!(
                "{}: {}",
                fixed(locale, "Reported check state", "보고된 검증 상태"),
                check.reported_state.label(locale)
            ),
            keys.clone(),
        ));
        let historical = answers.verification.as_ref().is_none_or(|selected| {
            selected.checkpoint_id.to_string() != check.original.checkpoint_id
        });
        result.facts.push(fact(if historical {"HistoricalVerification"} else {"VerificationObservation"},
            format!("{}: {} · {} · {}",fixed(locale,"Recorded verification quotation","기록된 검증 인용"),check.reported_state.label(locale),check.original_outcome.as_deref().unwrap_or(""),
                if matches!(check.scoped_run,crate::EvidenceDetail::Observed(_)) {fixed(locale,"Explicit scoped check report; comparison and declared coverage are separately qualified. No user acceptance is established.","명시적 범위 검증 보고; 비교와 선언된 coverage는 별도로 한정합니다. 사용자 수락은 미확인입니다.")} else {fixed(locale,"Assertion detail and comparable retest are not recorded; this report establishes no user acceptance or unrecorded coverage.","assertion 상세와 비교 가능한 재검증은 기록되지 않았습니다. 이 보고로 사용자 수락이나 미기록 coverage를 확정하지 않습니다.")}),keys));
    }
    for (field, state) in [
        ("UserReview", answers.review.as_ref()),
        ("UserAcceptance", answers.acceptance.as_ref()),
    ] {
        let label = match (field, state) {
            ("UserReview", Some(s)) => user_review_label(s.user_review.state, locale),
            (_, Some(s)) => user_acceptance_label(s.user_acceptance.state, locale),
            _ => match locale {
                FixedLocale::English => "No observation",
                FixedLocale::Korean => "관찰 없음",
            },
        };
        result.facts.push(fact(
            field,
            format!(
                "{}: {label}",
                if field == "UserReview" {
                    fixed(locale, "User review", "사용자 검토")
                } else {
                    fixed(locale, "User acceptance", "사용자 수락")
                }
            ),
            state.map_or_else(Vec::new, |s| {
                key(
                    s,
                    if field == "UserReview" {
                        "user_review"
                    } else {
                        "user_acceptance"
                    },
                )
            }),
        ));
    }
    if let Some(recorded) = answers
        .result
        .as_ref()
        .filter(|r| r.original_text.is_some() && result.provenance.is_none())
    {
        result.facts.push(fact(
            "ReportedResult",
            format!(
                "{}: {}",
                fixed(
                    locale,
                    "Recorded result quotation (original language)",
                    "기록된 결과 인용 (원문 언어)"
                ),
                recorded.original_text.as_deref().unwrap_or("")
            ),
            match &recorded.basis.record {
                crate::ReadingRecord::Checkpoint(id) => vec![format!(
                    "checkpoint:{id}@{}:{}",
                    recorded.basis.revision, recorded.basis.field
                )],
                _ => Vec::new(),
            },
        ));
    }
    if answers.result.is_none() {
        result.facts.push(fact(
            "ReportedResult",
            fixed(
                locale,
                "No reported result is recorded for this Work.",
                "이 작업에 보고된 결과가 기록되지 않았습니다.",
            ),
            Vec::new(),
        ));
    }
    if answers
        .verification
        .as_ref()
        .is_some_and(|s| !s.later_changed_checkpoint_ids.is_empty())
        || work.reading.states.iter().any(|s| {
            !s.later_changed_checkpoint_ids.is_empty()
                && s.verification
                    .iter()
                    .any(|v| v.state == volicord_context::VerificationState::Passed)
        })
    {
        result.facts.push(fact(
            "VerificationCoverage",
            fixed(
                locale,
                "Earlier verification is historical; coverage of later changes is not established.",
                "이전 검증은 과거 관찰입니다. 이후 변경의 검증 범위는 확인되지 않았습니다.",
            ),
            Vec::new(),
        ));
    }
    // Classify by exact independently selected observation, never display prose
    // or the last history position. A later empty verification does not make the
    // still-selected failure historical. Full observations remain in reading.states.
    let is_selected = |s: &WorkStateObservation, selected: &Option<WorkStateObservation>| {
        selected.as_ref().is_some_and(|current| {
            (s.checkpoint_id, s.checkpoint_revision)
                == (current.checkpoint_id, current.checkpoint_revision)
        })
    };
    let prior_failures = work
        .reading
        .states
        .iter()
        .filter(|s| {
            !is_selected(s, &answers.verification)
                && s.verification
                    .iter()
                    .any(|v| v.state == volicord_context::VerificationState::Failed)
        })
        .collect::<Vec<_>>();
    let prior_rejections = work
        .reading
        .states
        .iter()
        .filter(|s| {
            !is_selected(s, &answers.acceptance)
                && s.user_acceptance.state == volicord_context::UserAcceptanceState::Rejected
        })
        .collect::<Vec<_>>();
    let mut history = Vec::new();
    if !prior_failures.is_empty() {
        history.push(match locale {
            FixedLocale::English => format!(
                "verification failure observations: {}",
                prior_failures.len()
            ),
            FixedLocale::Korean => format!("검증 실패 관찰 {}건", prior_failures.len()),
        });
    }
    if !prior_rejections.is_empty() {
        history.push(match locale {
            FixedLocale::English => format!(
                "acceptance rejection observations: {}",
                prior_rejections.len()
            ),
            FixedLocale::Korean => format!("수락 거부 관찰 {}건", prior_rejections.len()),
        });
    }
    if !history.is_empty() {
        result.facts.push(fact(
            "HistoricalAdversity",
            format!("{}: {}. {}",
                fixed(locale, "Prior adverse history", "과거 문제 이력"),
                history.join("; "),
                fixed(locale,
                    "Current states are shown separately; inspect audit details for exact observations.",
                    "현재 상태와 구분되며 정확한 관찰은 감사 상세에서 확인하세요.")),
            prior_failures.into_iter().flat_map(|s| key(s, "verification"))
                .chain(prior_rejections.into_iter().flat_map(|s| key(s, "user_acceptance")))
                .collect(),
        ));
    }
    result
}

pub fn decision_answers(
    decision: &BriefDecision,
    language: &str,
    locale: FixedLocale,
) -> QuestionAnswers {
    let mut result = explanation_answers(&decision.explanations, language, locale);
    if let Some(gap) = source_gap(
        decision
            .user_source_status
            .iter()
            .chain(&decision.recommendation_source_status)
            .chain(&decision.question_source_status),
        locale,
    ) {
        result.facts.push(gap);
    }
    result.facts.push(fact(
        "QuestionContext",
        match &decision.question_context {
            Some(q) => format!(
                "{}: {}",
                fixed(
                    locale,
                    "Recorded question context quotation",
                    "기록된 질문 맥락 인용"
                ),
                q.prompt
            ),
            None => fixed(
                locale,
                "Exact originating question context is unavailable; no rationale is reconstructed.",
                "원래 질문의 정확한 revision 맥락을 사용할 수 없습니다. 이유를 복원하지 않습니다.",
            ),
        },
        vec![format!(
            "question:{}@{}:prompt_basis",
            decision.question_reference.question_id, decision.question_reference.revision
        )],
    ));
    result.facts.push(fact(
        "Choice",
        format!(
            "{}: {}",
            fixed(locale, "User choice", "사용자 선택"),
            crate::documents::decision_choice_attribution(decision, locale)
        ),
        vec![format!(
            "decision:{}@{}:choice",
            decision.decision_id, decision.revision
        )],
    ));
    result.facts.push(fact(
        "RecommendedChoice",
        format!(
            "{}: {}",
            fixed(locale, "Agent recommendation", "에이전트 권고"),
            crate::documents::recommendation_attribution(decision, locale)
        ),
        vec![format!(
            "decision:{}@{}:displayed_recommendation",
            decision.decision_id, decision.revision
        )],
    ));
    result.facts.push(fact(
        "DeclaredConsequences",
        format!(
            "{}: {}",
            fixed(locale, "Declared consequences", "선언된 결과"),
            crate::documents::alternative_consequences(decision, locale)
        ),
        vec![format!(
            "decision:{}@{}:displayed_alternatives",
            decision.decision_id, decision.revision
        )],
    ));
    result.facts.push(fact(
        "DecisionState",
        format!(
            "{}: {}",
            fixed(locale, "Decision state", "결정 상태"),
            crate::documents::brief_decision_state_label(decision.state, locale)
        ),
        vec![format!(
            "decision:{}@{}:lifecycle",
            decision.decision_id, decision.revision
        )],
    ));
    if decision
        .user_rationale
        .as_deref()
        .is_none_or(|r| r.trim().is_empty())
    {
        result.facts.push(fact(
            "UserRationale",
            fixed(
                locale,
                "User rationale is not recorded; agent recommendation does not supply it.",
                "사용자 근거가 기록되지 않았습니다. 에이전트 권고는 이를 대신하지 않습니다.",
            ),
            Vec::new(),
        ));
    }
    let scope = match decision.work_scope {
        volicord_context::DecisionWorkScope::ProjectWide => fixed(
            locale,
            "Declared scope: this Project; this does not establish applicability everywhere.",
            "선언 범위: 이 프로젝트. 모든 곳의 적용 가능성을 입증하지 않습니다.",
        ),
        volicord_context::DecisionWorkScope::WorkItem(id) => format!(
            "{}: {id}",
            fixed(locale, "Declared Work scope", "선언된 작업 범위")
        ),
        volicord_context::DecisionWorkScope::Unresolved => fixed(
            locale,
            "Declared Work scope is unresolved.",
            "선언된 작업 범위가 미해결입니다.",
        ),
    };
    result.facts.push(fact(
        "DeclaredScope",
        scope,
        vec![format!(
            "decision:{}@{}:work_scope",
            decision.decision_id, decision.revision
        )],
    ));
    for limit in &decision.known_limits {
        result.facts.push(fact(
            "KnownLimit",
            limit.clone(),
            vec![format!(
                "decision:{}@{}:known_limits",
                decision.decision_id, decision.revision
            )],
        ));
    }
    if !decision.review_issues.is_empty() {
        result.facts.push(fact(
            "ReviewBasis",
            decision.review_basis(locale).join("; "),
            vec![format!(
                "decision:{}@{}:review_basis",
                decision.decision_id, decision.revision
            )],
        ));
    }
    result
}

/// Internal current-build equality token. Never an authenticity or schema version.
pub fn canonical_read_fingerprint(canonical: &volicord_context::CanonicalReadBasis) -> String {
    use sha2::{Digest, Sha256};
    use std::hash::{Hash, Hasher};
    // Hash derives traverse every Eq field, including full source observations,
    // lifecycle and revision history. Avoid Debug escaping/identity formatting.
    // This token is deliberately current-build only, not a portable digest.
    struct DigestWriter {
        digest: Sha256,
        buffer: [u8; 8192],
        buffered: usize,
    }
    impl Hasher for DigestWriter {
        fn finish(&self) -> u64 {
            // Hash::hash only writes; the caller finalizes the full SHA-256.
            let mut digest = self.digest.clone();
            digest.update(&self.buffer[..self.buffered]);
            let bytes = digest.finalize();
            let mut prefix = [0_u8; 8];
            prefix.copy_from_slice(&bytes[..8]);
            u64::from_le_bytes(prefix)
        }
        fn write(&mut self, value: &[u8]) {
            if self.buffered + value.len() > self.buffer.len() {
                self.digest.update(&self.buffer[..self.buffered]);
                self.buffered = 0;
            }
            if value.len() >= 8192 {
                self.digest.update(value);
            } else {
                let end = self.buffered + value.len();
                self.buffer[self.buffered..end].copy_from_slice(value);
                self.buffered = end;
            }
        }
    }
    let mut writer = DigestWriter {
        digest: Sha256::new(),
        buffer: [0; 8192],
        buffered: 0,
    };
    canonical.hash(&mut writer);
    writer.digest.update(&writer.buffer[..writer.buffered]);
    format!("{:x}", writer.digest.finalize())
}

#[cfg(test)]
mod tests {
    use super::canonical_read_fingerprint;
    use sha2::{Digest, Sha256};
    use std::hash::{Hash, Hasher};
    use volicord_context::{CanonicalReadOptions, OperationId, Store};

    #[test]
    fn canonical_equality_digest_preserves_complete_hash_stream_at_buffer_boundaries(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Independently collect the Hash byte stream, without production buffering.
        #[derive(Default)]
        struct Stream(Vec<u8>);
        impl Hasher for Stream {
            fn write(&mut self, bytes: &[u8]) {
                self.0.extend_from_slice(bytes);
            }
            fn finish(&self) -> u64 {
                let bytes = Sha256::digest(&self.0);
                let mut prefix = [0; 8];
                prefix.copy_from_slice(&bytes[..8]);
                u64::from_le_bytes(prefix)
            }
        }
        let temp = tempfile::tempdir()?;
        let mut store = Store::open(temp.path().join("canonical.sqlite3"))?;
        let project = store
            .create_project(OperationId::from_bytes([1; 16]), "Digest boundary")?
            .value;
        let mut canonical =
            store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
        for length in [0, 1, 8191, 8192, 8193, 16383, 16384, 65536] {
            canonical.project.display_name = format!("{}한🙂", "x".repeat(length));
            let mut stream = Stream::default();
            canonical.hash(&mut stream);
            let original = canonical_read_fingerprint(&canonical);
            assert_eq!(original, format!("{:x}", Sha256::digest(&stream.0)));
            // An equal-size change at the end cannot be lost at a flush boundary.
            canonical.project.display_name.pop();
            canonical.project.display_name.push('🙃');
            assert_ne!(original, canonical_read_fingerprint(&canonical));
        }
        Ok(())
    }
}
