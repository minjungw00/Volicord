//! Question-scoped active-host Work interpretation. Plans contain canonical
//! evidence, never a prewritten answer. Recording validates grounding structure,
//! not prose truth, authorship, language comprehension or implementation success.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_context::{CanonicalReadBasis, ContextItemId, ContextItemRole};

pub const WORK_EXPLANATION_KIND: &str = "volicord_work_explanation";
pub const WORK_EXPLANATION_VERSION: u32 = 1;
pub const WORK_EXPLANATION_BYTE_LIMIT: usize = 16_384;
pub const WORK_EXPLANATION_PLAN_BYTE_LIMIT: usize = 131_072;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkExplanationQuestion {
    Purpose,
    ReportedChange,
    ExpectedEffect,
    Verification,
    NextStep,
    Limits,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkExplanationEvidence {
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
pub struct WorkExplanationPlan {
    pub project_id: String,
    pub work_item_id: String,
    pub question: String,
    pub requested_language: String,
    pub evidence: Vec<WorkExplanationEvidence>,
    pub source_status: Vec<Value>,
    pub conflicts: Vec<Value>,
    pub instructions: String,
    pub fingerprint: String,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkExplanationGenerator {
    pub host: String,
    pub session: String,
    pub agent: Option<String>,
    pub model: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkExplanationParagraph {
    pub question: WorkExplanationQuestion,
    pub text: String,
    pub evidence_keys: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkExplanationRealization {
    pub format_kind: String,
    pub format_version: u32,
    pub plan_fingerprint: String,
    pub language: String,
    pub generator: WorkExplanationGenerator,
    pub paragraphs: Vec<WorkExplanationParagraph>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedWorkExplanation {
    pub realization: WorkExplanationRealization,
    pub project_id: String,
    pub work_item_id: String,
    pub question: String,
    /// Evidence identity/revision/field, without duplicating original content.
    pub evidence: Vec<WorkExplanationEvidence>,
    pub source_status: Vec<Value>,
    pub conflicts: Vec<Value>,
    pub generated_at_unix_micros: i64,
    /// Always assigned by the recorder. Caller model strings are not attestations.
    pub generator_identity_status: String,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkExplanationState {
    Current,
    Stale,
    Unavailable,
    Unsupported,
    Corrupt,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkExplanationReading {
    pub language: String,
    pub state: WorkExplanationState,
    pub content: Option<RetainedWorkExplanation>,
    pub diagnostic: Option<String>,
}

pub fn prepare_work_explanation(
    canonical: &CanonicalReadBasis,
    work: ContextItemId,
    language: &str,
) -> Result<WorkExplanationPlan, String> {
    if language.trim().is_empty() || language.len() > 128 || language.chars().any(char::is_control)
    {
        return Err("a bounded nonempty requested language is required".into());
    }
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
        evidence.push(WorkExplanationEvidence {
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
            let mut sources = s.work_source_basis.clone();
            sources.extend(s.verification.iter().filter_map(|v| v.source_id));
            sources.extend(s.user_review.source_id);
            sources.extend(s.user_acceptance.source_id);
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
    let mut plan = WorkExplanationPlan { project_id:canonical.project.id.to_string(), work_item_id:work.to_string(),
        question:"work_outcome".into(), requested_language:language.into(), evidence, source_status, conflicts,
        instructions:"Explain purpose, reported change, expected effect, verification limits and next step in the requested language. Interpret clear source prose; do not copy audit clutter into ordinary reading. Cite exact evidence keys per paragraph. Report missing information explicitly. A generic implementation-changed report supports no specific feature. A reported change is not independently verified success. Separate work, verification, review and acceptance; later changes are not covered silently. State contradictions and uncertainty. Do not infer runtime code behavior or user rationale. Generator identity is self-reported. Return WorkExplanationRealization JSON, format_kind volicord_work_explanation, format_version 1, this plan fingerprint and language, generator host/session/agent/model (null when unknown), and paragraphs question/text/evidence_keys.".into(), fingerprint:String::new() };
    let bytes = serde_json::to_vec(&plan).map_err(|e| e.to_string())?;
    if bytes.len() > WORK_EXPLANATION_PLAN_BYTE_LIMIT {
        return Err("explanation preparation exceeds evidence budget; no source text was silently truncated".into());
    }
    plan.fingerprint = format!("sha256:{:x}", Sha256::digest(bytes));
    Ok(plan)
}

pub fn validate_work_explanation(
    plan: &WorkExplanationPlan,
    response: &WorkExplanationRealization,
) -> Result<(), String> {
    if response.format_kind != WORK_EXPLANATION_KIND
        || response.format_version != WORK_EXPLANATION_VERSION
    {
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
        > WORK_EXPLANATION_BYTE_LIMIT
    {
        return Err("Work explanation exceeds body budget".into());
    }
    for question in [
        WorkExplanationQuestion::Purpose,
        WorkExplanationQuestion::ReportedChange,
        WorkExplanationQuestion::ExpectedEffect,
        WorkExplanationQuestion::Verification,
        WorkExplanationQuestion::NextStep,
    ] {
        if response
            .paragraphs
            .iter()
            .filter(|p| p.question == question)
            .count()
            != 1
        {
            return Err("one answer for each required Work question is necessary".into());
        }
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
            WorkExplanationQuestion::Purpose => "goal",
            WorkExplanationQuestion::ReportedChange | WorkExplanationQuestion::ExpectedEffect => {
                if plan.evidence.iter().any(|e| e.key == "result") {
                    "result"
                } else {
                    "goal"
                }
            }
            WorkExplanationQuestion::Verification => {
                if plan.evidence.iter().any(|e| e.key == "verification") {
                    "verification"
                } else {
                    "goal"
                }
            }
            WorkExplanationQuestion::NextStep => "next_step",
            WorkExplanationQuestion::Limits => continue,
        };
        if !p.evidence_keys.iter().any(|key| key == required) {
            return Err(format!("answer {:?} must reference {required}", p.question));
        }
    }
    Ok(())
}
