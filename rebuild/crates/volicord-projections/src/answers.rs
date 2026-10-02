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
    pub generated_at_unix_micros: i64,
    pub generator: ExplanationGenerator,
    pub generator_identity_status: String,
    pub evidence: Vec<ExplanationEvidence>,
    pub source_status: Vec<serde_json::Value>,
    pub conflicts: Vec<serde_json::Value>,
}
impl From<&RetainedExplanation> for ExplanationProvenance {
    fn from(e: &RetainedExplanation) -> Self {
        Self {
            project_id: e.project_id.clone(),
            subject: e.subject,
            question: e.question.clone(),
            language: e.realization.language.clone(),
            fingerprint: e.realization.plan_fingerprint.clone(),
            generated_at_unix_micros: e.generated_at_unix_micros,
            generator: e.realization.generator.clone(),
            generator_identity_status: e.generator_identity_status.clone(),
            evidence: e.evidence.clone(),
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_action: Option<RecordedActionBasis>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct QuestionAnswers {
    pub prose: Vec<QuestionAnswer>,
    pub facts: Vec<QuestionAnswer>,
    pub explanation_state: ExplanationState,
    pub diagnostic: Option<String>,
    pub provenance: Option<ExplanationProvenance>,
}
impl QuestionAnswers {
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
        self.prose
            .iter()
            .chain(&self.facts)
            .map(|a| a.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
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
    let state = reading.map_or(ExplanationState::Unavailable, |e| e.state);
    let current = reading
        .filter(|e| e.state == ExplanationState::Current)
        .and_then(|e| e.content.as_ref());
    let prose = if let Some(e) = current {
        e.realization
            .paragraphs
            .iter()
            .map(|p| QuestionAnswer {
                question: format!("{:?}", p.question),
                text: p.text.clone(),
                role: AnswerRole::GeneratedInterpretation,
                evidence_keys: p.evidence_keys.clone(),
                recorded_action: None,
            })
            .collect()
    } else {
        let (en,ko)=match state {
            ExplanationState::Stale=>("Explanation is stale; prepare and generate again.","설명의 근거가 변경되었습니다. 다시 준비하고 생성하세요."),
            ExplanationState::Corrupt=>("Explanation is corrupt; delete and regenerate.","설명이 손상되었습니다. 삭제한 뒤 다시 생성하세요."),
            ExplanationState::Unsupported=>("Explanation format is unsupported; delete and regenerate.","설명 형식을 지원하지 않습니다. 삭제한 뒤 다시 생성하세요."),
            _ if reading.is_some()=>("Explanation is unavailable; canonical facts remain readable. Inspect the diagnostic for the affected dependency.","설명을 사용할 수 없습니다. Canonical 사실은 계속 읽을 수 있습니다. 영향을 받은 의존성은 진단에서 확인하세요."),
            _=>("Interpretation has not been generated in this language. Prepare evidence and ask the active host to explain it.","이 언어의 해석이 아직 생성되지 않았습니다. 근거를 준비하고 현재 호스트에 설명을 요청하세요."),
        };
        vec![QuestionAnswer {
            question: "ExplanationAvailability".into(),
            text: fixed(locale, en, ko),
            role: AnswerRole::Unavailable,
            evidence_keys: Vec::new(),
            recorded_action: None,
        }]
    };
    let mut result = QuestionAnswers {
        prose,
        facts: Vec::new(),
        explanation_state: state,
        diagnostic: reading.and_then(|e| e.diagnostic.clone()),
        provenance: current.map(Into::into),
    };
    if current.is_some_and(|e| !e.conflicts.is_empty()) {
        result.facts.push(fact("Conflicts",fixed(locale,"The evidence includes contradiction or supersession; this interpretation does not resolve it.","근거에 모순 또는 대체 관계가 있습니다. 이 해석은 해당 관계를 해결하지 않습니다."),Vec::new()));
    }
    result
}
fn fact(question: &str, text: String, evidence_keys: Vec<String>) -> QuestionAnswer {
    QuestionAnswer {
        question: question.into(),
        text,
        role: AnswerRole::DeterministicFacts,
        evidence_keys,
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
    let verification = answers.verification.as_ref().map_or_else(
        || fixed(locale, "No verification record", "검증 기록 없음"),
        |s| {
            s.verification
                .iter()
                .map(|v| verification_state_label(v.state, locale))
                .collect::<Vec<_>>()
                .join(", ")
        },
    );
    result.facts.push(fact(
        "VerificationState",
        format!(
            "{}: {verification}",
            fixed(locale, "Automated verification", "자동 검증")
        ),
        answers
            .verification
            .as_ref()
            .map_or_else(Vec::new, |s| key(s, "verification")),
    ));
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
    for s in &work.reading.states {
        if s.verification
            .iter()
            .any(|v| v.state == volicord_context::VerificationState::Failed)
            || s.user_acceptance.state == volicord_context::UserAcceptanceState::Rejected
        {
            result.facts.push(fact(
                "AdverseObservation",
                format!(
                    "{}: {} / {} ({})",
                    fixed(
                        locale,
                        "Recorded failure / acceptance history",
                        "기록된 실패 / 수락 이력"
                    ),
                    s.verification
                        .iter()
                        .map(|v| verification_state_label(v.state, locale))
                        .collect::<Vec<_>>()
                        .join(", "),
                    user_acceptance_label(s.user_acceptance.state, locale),
                    s.observed_at.as_unix_micros()
                ),
                key(s, "verification,user_acceptance"),
            ));
        }
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
            .chain(&decision.recommendation_source_status),
        locale,
    ) {
        result.facts.push(gap);
    }
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
    use std::fmt::Write;
    // Preserve the exact current-build Debug byte stream while avoiding a
    // second complete prose buffer. Batch tiny formatting writes before hashing.
    struct DigestWriter {
        digest: Sha256,
        buffer: Vec<u8>,
    }
    impl Write for DigestWriter {
        fn write_str(&mut self, value: &str) -> std::fmt::Result {
            if self.buffer.len() + value.len() > 8192 {
                self.digest.update(&self.buffer);
                self.buffer.clear();
            }
            if value.len() >= 8192 {
                self.digest.update(value.as_bytes());
            } else {
                self.buffer.extend_from_slice(value.as_bytes());
            }
            Ok(())
        }
    }
    let mut writer = DigestWriter {
        digest: Sha256::new(),
        buffer: Vec::with_capacity(8192),
    };
    // This writer cannot fail; hashing is an ephemeral equality check, not a
    // durable-state transition. No complete string is built on failure.
    let _ = write!(&mut writer, "{canonical:?}");
    writer.digest.update(&writer.buffer);
    format!("{:x}", writer.digest.finalize())
}
