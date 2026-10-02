//! Question-scoped active-host Work interpretation. Plans contain canonical
//! evidence, never a prewritten answer. Recording validates grounding structure,
//! not prose truth, authorship, language comprehension or implementation success.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_context::{CanonicalReadBasis, ContextItemId, ContextItemRole, DecisionId};

pub const EXPLANATION_KIND: &str = "volicord_explanation";
pub const EXPLANATION_VERSION: u32 = 1;
pub const EXPLANATION_BYTE_LIMIT: usize = 16_384;
pub const EXPLANATION_PLAN_BYTE_LIMIT: usize = 131_072;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "identity", rename_all = "snake_case")]
pub enum ExplanationSubject {
    Work(ContextItemId),
    Decision(DecisionId),
}
impl ExplanationSubject {
    pub fn key(self) -> String {
        match self {
            Self::Work(id) => format!("work:{id}"),
            Self::Decision(id) => format!("decision:{id}"),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplanationQuestion {
    Purpose,
    ReportedChange,
    ExpectedEffect,
    Verification,
    NextStep,
    Limits,
    UserRationale,
    Recommendation,
    Consequences,
    Applicability,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExplanationEvidence {
    pub key: String,
    pub record_kind: String,
    pub identity: String,
    pub revision: u64,
    pub field: String,
    pub sources: Vec<String>,
    pub content: Value,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExplanationPlan {
    pub project_id: String,
    pub subject: ExplanationSubject,
    pub question: String,
    pub requested_language: String,
    pub evidence: Vec<ExplanationEvidence>,
    pub source_status: Vec<Value>,
    pub conflicts: Vec<Value>,
    pub instructions: String,
    pub fingerprint: String,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExplanationGenerator {
    pub host: String,
    pub session: String,
    pub agent: Option<String>,
    pub model: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExplanationParagraph {
    pub question: ExplanationQuestion,
    pub text: String,
    pub evidence_keys: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExplanationRealization {
    pub format_kind: String,
    pub format_version: u32,
    pub plan_fingerprint: String,
    pub language: String,
    pub generator: ExplanationGenerator,
    pub paragraphs: Vec<ExplanationParagraph>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedExplanation {
    pub realization: ExplanationRealization,
    pub project_id: String,
    pub subject: ExplanationSubject,
    pub question: String,
    /// Evidence identity/revision/field, without duplicating original content.
    pub evidence: Vec<ExplanationEvidence>,
    pub source_status: Vec<Value>,
    pub conflicts: Vec<Value>,
    pub generated_at_unix_micros: i64,
    /// Always assigned by the recorder. Caller model strings are not attestations.
    pub generator_identity_status: String,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplanationState {
    Current,
    Stale,
    Unavailable,
    Unsupported,
    Corrupt,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExplanationReading {
    pub language: String,
    pub state: ExplanationState,
    pub content: Option<RetainedExplanation>,
    pub diagnostic: Option<String>,
}

pub fn prepare_explanation(
    canonical: &CanonicalReadBasis,
    subject: ExplanationSubject,
    language: &str,
) -> Result<ExplanationPlan, String> {
    if language.trim().is_empty() || language.len() > 128 || language.chars().any(char::is_control)
    {
        return Err("a bounded nonempty requested language is required".into());
    }
    let work = match subject {
        ExplanationSubject::Work(work) => work,
        ExplanationSubject::Decision(decision) => {
            return prepare_decision(canonical, decision, language)
        }
    };
    let scoped = crate::reading::scope_to_work(canonical, work);
    let selected = crate::reading::derive_work_history(&scoped)
        .into_iter()
        .find(|w| w.work_item_id == work)
        .ok_or("Work not found in this Project")?;
    let mut evidence = Vec::new();
    let mut add = |key: &str, basis: &crate::ReadingBasis, content: Value| {
        let (kind, identity) = match basis.record {
            crate::ReadingRecord::ContextItem(id) => ("context_item", id.to_string()),
            crate::ReadingRecord::Checkpoint(id) => ("checkpoint", id.to_string()),
            crate::ReadingRecord::Decision(id) => ("decision", id.to_string()),
        };
        evidence.push(ExplanationEvidence {
            key: key.into(),
            record_kind: kind.into(),
            identity,
            revision: basis.revision,
            field: basis.field.clone(),
            sources: basis.source_basis.iter().map(ToString::to_string).collect(),
            content,
        });
    };
    add("goal", &selected.reading.goal.basis, json!(selected.title));
    if let Some(result) = &selected.reading.answers.result {
        add("result", &result.basis, json!(result.original_text));
    }
    add(
        "next_step",
        &selected.reading.next_step.basis,
        json!(selected.next_step),
    );
    let mut state = |key: &str, observation: &Option<crate::WorkStateObservation>| {
        if let Some(s) = observation {
            let sources = match key {
                "latest_state" => s.work_source_basis.clone(),
                "verification" => s.verification.iter().filter_map(|v| v.source_id).collect(),
                "review" => s.user_review.source_id.into_iter().collect(),
                "acceptance" => s.user_acceptance.source_id.into_iter().collect(),
                _ => s
                    .work_source_basis
                    .iter()
                    .copied()
                    .chain(s.verification.iter().filter_map(|v| v.source_id))
                    .chain(s.user_review.source_id)
                    .chain(s.user_acceptance.source_id)
                    .collect(),
            };
            let basis = crate::reading::reading_basis(
                canonical,
                crate::ReadingRecord::Checkpoint(s.checkpoint_id),
                s.checkpoint_revision,
                match key {
                    "latest_state" => "work_state",
                    "verification" => "verification",
                    "review" => "user_review",
                    "acceptance" => "user_acceptance",
                    _ => "work_state,verification,user_review,user_acceptance",
                },
                sources,
            );
            add(
                key,
                &basis,
                json!({"observed_at":s.observed_at.as_unix_micros(),
                "work_state":format!("{:?}",s.work_state),
                "verification":s.verification.iter().map(|v|json!({"state":format!("{:?}",v.state),"source":v.source_id.map(|id|id.to_string()),"outcome":v.outcome})).collect::<Vec<_>>(),
                "review":format!("{:?}",s.user_review.state),"acceptance":format!("{:?}",s.user_acceptance.state),
                "later_changes":s.later_changed_checkpoint_ids.iter().map(ToString::to_string).collect::<Vec<_>>()}),
            );
        }
    };
    state("latest_state", &selected.reading.answers.latest_state);
    state("verification", &selected.reading.answers.verification);
    state("review", &selected.reading.answers.review);
    state("acceptance", &selected.reading.answers.acceptance);
    for observation in &selected.reading.states {
        state(
            &format!("history:{}", observation.checkpoint_id),
            &Some(observation.clone()),
        );
    }
    for cp in canonical
        .checkpoint_history
        .iter()
        .filter(|cp| cp.project_id == canonical.project.id && cp.work_item_id == Some(work))
    {
        let basis = crate::reading::reading_basis(
            canonical,
            crate::ReadingRecord::Checkpoint(cp.id),
            cp.revision,
            "known_limits",
            cp.source_basis.clone(),
        );
        add(
            &format!("limits:{}", cp.id),
            &basis,
            json!({"known_limits":cp.known_limits,"non_goals":cp.non_goals}),
        );
    }
    for purpose in canonical.context_items.iter().filter(|c| {
        c.project_id == canonical.project.id && c.role == ContextItemRole::ProjectPurpose
    }) {
        let basis = crate::reading::reading_basis(
            canonical,
            crate::ReadingRecord::ContextItem(purpose.id),
            purpose.revision,
            "statement",
            purpose.source_basis.clone(),
        );
        add(
            &format!("project_purpose:{}", purpose.id),
            &basis,
            json!(purpose.statement),
        );
    }
    finish_plan(canonical, subject, "work_outcome", language, evidence)
}

fn prepare_decision(
    canonical: &CanonicalReadBasis,
    id: DecisionId,
    language: &str,
) -> Result<ExplanationPlan, String> {
    let lifecycle = canonical
        .active_decisions
        .iter()
        .chain(&canonical.superseded_decisions)
        .find(|d| d.decision.id == id && d.decision.project_id == canonical.project.id)
        .ok_or("Decision not found in this Project")?;
    let decision = crate::recall::brief_decision(
        canonical,
        lifecycle,
        &volicord_inquiry::ApplicabilityQuery {
            current_assumptions: Vec::new(),
            met_revisit_triggers: Vec::new(),
            project_id: canonical.project.id,
            paths: Vec::new(),
            components: Vec::new(),
            work_contexts: Vec::new(),
        },
    );
    let evidence = [
        ("choice", "choice", decision.user_source_basis.clone(), json!({"choice":format!("{:?}",decision.choice),"chosen_alternative_key":decision.chosen_alternative_key,"state":format!("{:?}",decision.state)})),
        ("user_rationale", "user_rationale", decision.user_source_basis.clone(), json!(decision.user_rationale)),
        ("recommendation", "displayed_recommendation", decision.recommendation_source_basis.clone(), json!({"alternative_key":decision.recommended_alternative_key,"rationale":decision.recommendation_rationale})),
        ("consequences", "displayed_alternatives", decision.recommendation_source_basis.clone(), json!(decision.displayed_alternatives.iter().map(|a|json!({"key":a.key,"label":a.label,"consequence":a.consequence})).collect::<Vec<_>>())),
        ("applicability", "applicability", decision.source_basis.clone(), json!({"scope":format!("{:?}",decision.work_scope),"paths":lifecycle.decision.applicability.paths,"components":lifecycle.decision.applicability.components,"work_contexts":lifecycle.decision.applicability.work_contexts,"assumptions":decision.assumptions,"revisit_triggers":decision.revisit_triggers,"known_limits":decision.known_limits,"review_basis":decision.review_basis(crate::FixedLocale::English)})),
    ].into_iter().map(|(key,field,sources,content)| ExplanationEvidence {
        key:key.into(),field:field.into(),sources:sources.into_iter().map(|id|id.to_string()).collect(),content,
        record_kind:"decision".into(),identity:id.to_string(),revision:decision.revision,
    }).collect();
    finish_plan(
        canonical,
        ExplanationSubject::Decision(id),
        "decision_rationale",
        language,
        evidence,
    )
}

fn finish_plan(
    canonical: &CanonicalReadBasis,
    subject: ExplanationSubject,
    question: &str,
    language: &str,
    evidence: Vec<ExplanationEvidence>,
) -> Result<ExplanationPlan, String> {
    let sources = evidence
        .iter()
        .flat_map(|e| e.sources.iter())
        .collect::<std::collections::BTreeSet<_>>();
    let source_status = canonical.sources.iter().filter(|s|sources.contains(&s.source.id.to_string()))
        .map(|s|json!({"identity":s.source.id.to_string(),"immutable":true, "recorded_at":s.source.recorded_at.as_unix_micros(),
            "snapshot":s.snapshot_basis,"availability":format!("{:?}",s.availability),"freshness":format!("{:?}",s.freshness),
            "actor":format!("{:?}",s.source.actor),"observer":format!("{:?}",s.source.observer),
            "observation":format!("{:?}",s.source.payload)})).collect();
    let identities = evidence
        .iter()
        .map(|e| e.identity.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let conflicts = canonical
        .relations
        .iter()
        .filter(|r| {
            (identities.contains(r.from_identity.as_str())
                || identities.contains(r.to_identity.as_str()))
                && matches!(r.relation_kind.as_str(), "contradicts" | "supersedes")
        })
        .map(|r| json!({"from":r.from_identity,"relation":r.relation_kind,"to":r.to_identity}))
        .collect();
    let questions = match subject {
        ExplanationSubject::Work(_) => "Answer purpose, reported_change, expected_effect, verification and next_step (optional limits).",
        ExplanationSubject::Decision(_) => "Answer user_rationale, recommendation, consequences and applicability (optional limits). Missing user rationale stays missing; agent rationale never supplies it.",
    };
    let mut plan = ExplanationPlan { project_id:canonical.project.id.to_string(), subject,
        question:question.into(), requested_language:language.into(), evidence, source_status, conflicts,
        instructions:format!("{questions} Interpret full source prose in the requested language, not audit clutter. Cite exact evidence keys. Keep checksums when they are subject matter. State missing information, contradictions and uncertainty. Generic implementation-changed prose supports no specific feature. Reports are not independently verified success. Separate work, verification, review and acceptance; earlier checks do not cover later changes. Do not invent user rationale or runtime behavior. Return ExplanationRealization JSON, format_kind volicord_explanation, format_version 1, exact fingerprint/language, self-reported generator host/session/agent/model (null when unknown), and paragraphs question/text/evidence_keys."), fingerprint:String::new() };
    let bytes = serde_json::to_vec(&plan).map_err(|e| e.to_string())?;
    if bytes.len() > EXPLANATION_PLAN_BYTE_LIMIT {
        return Err("explanation preparation exceeds evidence budget; no source text was silently truncated".into());
    }
    plan.fingerprint = format!("sha256:{:x}", Sha256::digest(bytes));
    Ok(plan)
}

pub fn validate_explanation(
    plan: &ExplanationPlan,
    response: &ExplanationRealization,
) -> Result<(), String> {
    if response.format_kind != EXPLANATION_KIND || response.format_version != EXPLANATION_VERSION {
        return Err(
            "unsupported Work explanation format; regenerate from current preparation".into(),
        );
    }
    if response.plan_fingerprint != plan.fingerprint || response.language != plan.requested_language
    {
        return Err("stale preparation or language mismatch; prepare again".into());
    }
    if response.generator.host.trim().is_empty() || response.generator.session.trim().is_empty() {
        return Err("active host and session provenance are required".into());
    }
    if serde_json::to_vec(response)
        .map_err(|e| e.to_string())?
        .len()
        > EXPLANATION_BYTE_LIMIT
    {
        return Err("Work explanation exceeds body budget".into());
    }
    let required_questions: &[ExplanationQuestion] = match plan.subject {
        ExplanationSubject::Work(_) => &[
            ExplanationQuestion::Purpose,
            ExplanationQuestion::ReportedChange,
            ExplanationQuestion::ExpectedEffect,
            ExplanationQuestion::Verification,
            ExplanationQuestion::NextStep,
        ],
        ExplanationSubject::Decision(_) => &[
            ExplanationQuestion::UserRationale,
            ExplanationQuestion::Recommendation,
            ExplanationQuestion::Consequences,
            ExplanationQuestion::Applicability,
        ],
    };
    for question in required_questions {
        if response
            .paragraphs
            .iter()
            .filter(|p| &p.question == question)
            .count()
            != 1
        {
            return Err("one answer for each required question is necessary".into());
        }
    }
    if response.paragraphs.iter().any(|p| {
        p.question != ExplanationQuestion::Limits && !required_questions.contains(&p.question)
    }) {
        return Err("answer question does not belong to prepared subject".into());
    }
    for p in &response.paragraphs {
        if p.text.trim().is_empty()
            || p.evidence_keys.is_empty()
            || p.evidence_keys
                .iter()
                .any(|key| !plan.evidence.iter().any(|e| &e.key == key))
        {
            return Err(
                "each answer requires nonempty text and exact prepared evidence keys".into(),
            );
        }
        let required = match p.question {
            ExplanationQuestion::Purpose => "goal",
            ExplanationQuestion::ReportedChange | ExplanationQuestion::ExpectedEffect => {
                if plan.evidence.iter().any(|e| e.key == "result") {
                    "result"
                } else {
                    "goal"
                }
            }
            ExplanationQuestion::Verification => {
                if plan.evidence.iter().any(|e| e.key == "verification") {
                    "verification"
                } else {
                    "goal"
                }
            }
            ExplanationQuestion::NextStep => "next_step",
            ExplanationQuestion::UserRationale => "user_rationale",
            ExplanationQuestion::Recommendation => "recommendation",
            ExplanationQuestion::Consequences => "consequences",
            ExplanationQuestion::Applicability => "applicability",
            ExplanationQuestion::Limits => continue,
        };
        if !p.evidence_keys.iter().any(|key| key == required) {
            return Err(format!("answer {:?} must reference {required}", p.question));
        }
    }
    Ok(())
}
