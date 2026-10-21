//! Question-scoped active-host Work interpretation. Plans contain canonical
//! evidence, never a prewritten answer. Recording validates grounding structure,
//! not prose truth, authorship, language comprehension or implementation success.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_context::{CanonicalReadBasis, ContextItemId, ContextItemRole, DecisionId};

pub const EXPLANATION_KIND: &str = "volicord_explanation";
pub const EXPLANATION_VERSION: u32 = 6;
/// Complete claim bindings and prose share this limit; retention stays independent.
pub const EXPLANATION_BYTE_LIMIT: usize = 40_960;
/// Preserve the response space guaranteed before citation metadata grew.
pub const EXPLANATION_MIN_RESPONSE_BYTE_CAPACITY: usize = 16_384;
pub const EXPLANATION_PARAGRAPH_CHARACTER_LIMIT: usize = 16_384;
pub const EXPLANATION_PLAN_BYTE_LIMIT: usize = 131_072;
/// Independent bound for the complete compact retained envelope.
pub const EXPLANATION_RETAINED_BYTE_LIMIT: usize = 147_456;
/// File transport allows formatting; compact realization admission is independent.
pub const EXPLANATION_INPUT_BYTE_LIMIT: usize = 65_536;
const GENERATOR_IDENTITY_STATUS: &str = "self_reported_not_independently_verified";

/// One authoring contract for CLI/MCP preparation, independent of transport.
/// These are investigation obligations, never prewritten subject answers.
const ARTICLE_AUTHORING: &str = "Read original sources first. Identify missing explanatory steps and group actual topics with section_heading followed by supporting narrative/comparison/flow. Use useful authored headings at topic changes, not an undifferentiated multi-topic paragraph stream. Short single-topic answers may be unheaded; no fixed names, count or enum order. No empty headings. Explain conditions -> relevant implementation -> consequence. Distinguish structural inference, recorded rationale and observed execution. Corrections need evidenced symptom/conditions, cause, actual modification, mechanism and positive/negative consequences. Missing before-state remains unknown. No-change/preferences invent no cause, patch, retry, tests or Decision. Write a concise lead and substantive prose without repeating the card or 'the user asked'/'the report says'. Localize material uncertainty; consolidate equivalent cautions. Use verification/limits roles for separate reading layers; a layer change ends a section. Read claims against exact Sources before compose. Admission is not semantic or human approval.";

/// Repository investigation is independent of a Work and precedes authoring.
/// Inventory commitments are discovery inputs, not an architectural answer key.
const REPOSITORY_AUTHORING: &str = "Investigate repository-wide before authoring. First inspect exact analysis evidence, README/configuration and entry points; independently identify material responsibilities and explanatory questions from the actual repository, not a recent Work or inventory counts. Within existing source authority explicitly observe/read selected implementations, callers/callees, storage/transport boundaries and relevant tests across those responsibilities. Follow important cross-boundary relations until their mechanism and failure/exit paths are supported or explicitly unknown; do not stop after the first well-explained subsystem. Distinguish inventory coverage, analyzed topology, actually investigated Sources, explained topics and unknown/omitted areas. Then author a small integrative overview: what system this is, who does what, how responsibilities cooperate and why their boundaries matter where evidence supports it. Explain at least two materially distinct end-to-end operations when repository complexity warrants them; a genuinely single-purpose small repository need not invent a second flow. Use supported narrative units for flows; no fixed section names/count, file catalog or mandatory empty sections. Detail demonstrates the broader account rather than defining the repository by one operation. Keep short truthful scope and material unresolved failures near the opening; put routine inventory/provenance in supporting inspection. Use section_heading followed by substantive content at actual topic changes. Review every technical claim, including assertive headings/synopsis, against relevant exact Sources; a matching hash or a heading role proves no entailment. State unsupported responsibilities and missing causal bridges rather than fabricate topology, execution, rationale, Work or canonical Project Purpose. Before compose review breadth and causal coverage separately from bytes; preserve necessary citations. If a necessary explanation cannot fit, report the exact unrepresentable obligation and measured preparation/response/retained pressure; do not silently omit it or raise limits. Admission is not semantic or human approval.";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExplanationRetentionBudget {
    pub response_byte_limit: usize,
    pub retained_byte_limit: usize,
    /// Exact compact envelope overhead with the longest possible i64 timestamp.
    pub metadata_byte_reserve: usize,
    pub response_byte_capacity: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "identity", rename_all = "snake_case")]
pub enum ExplanationSubject {
    Work(ContextItemId),
    /// Independent publications of the same canonical Work, never new entities.
    WorkName(ContextItemId),
    WorkSummary(ContextItemId),
    Decision(DecisionId),
    Repository(volicord_repository_intelligence::AnalysisSnapshotId),
}
impl ExplanationSubject {
    pub fn key(self) -> String {
        match self {
            Self::Work(id) => format!("work:{id}"),
            Self::WorkName(id) => format!("work_name:{id}"),
            Self::WorkSummary(id) => format!("work_summary:{id}"),
            Self::Decision(id) => format!("decision:{id}"),
            Self::Repository(id) => format!("repository:{id}"),
        }
    }
    pub fn work(self) -> Option<ContextItemId> {
        match self {
            Self::Work(id) | Self::WorkName(id) | Self::WorkSummary(id) => Some(id),
            _ => None,
        }
    }
    pub fn work_publication(work: ContextItemId, scope: &str) -> Result<Self, String> {
        match scope {
            "article" => Ok(Self::Work(work)),
            "name" => Ok(Self::WorkName(work)),
            "summary" => Ok(Self::WorkSummary(work)),
            _ => Err("Work publication scope must be article, name or summary".into()),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplanationQuestion {
    /// Flat editorial units: each visible fragment has its own admitted basis.
    SectionHeading,
    Narrative,
    ComparisonCondition,
    ComparisonResult,
    FlowStep,
    /// Authored derived wording, never a canonical Work name or extracted Goal.
    Heading,
    Synopsis,
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
    Observation,
    Comparison,
    ReadingPoint,
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
/// Admission requirements, not a claim that any listed key entails arbitrary
/// prose. Source citations independently support technical claims.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExplanationQuestionGrounding {
    pub question: ExplanationQuestion,
    /// At least one is required in each unit for this role. An empty role rule
    /// still requires relevant exact prepared evidence (e.g. heading or limits).
    pub required_any_evidence_keys: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExplanationPlan {
    pub project_id: String,
    pub subject: ExplanationSubject,
    pub question: String,
    pub requested_language: String,
    pub evidence: Vec<ExplanationEvidence>,
    pub question_grounding: Vec<ExplanationQuestionGrounding>,
    pub source_status: Vec<Value>,
    pub conflicts: Vec<Value>,
    pub instructions: String,
    pub retention_budget: ExplanationRetentionBudget,
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
    pub citations: Vec<ExplanationCitation>,
}

/// File evidence uses absolute repository-file byte/line coordinates, including
/// bounded observations. Command fields and paragraph claims use field-relative bytes.
/// Byte ranges are half-open UTF-8 ranges.
/// Hash equality binds wording to bytes; it does not establish entailment.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExplanationCitation {
    pub source_id: String,
    pub source_revision: u64,
    pub observation_fingerprint: String,
    pub analysis_snapshot_id: String,
    pub repository_snapshot_id: String,
    pub repository_source_id: String,
    pub locator: String,
    pub start_line: usize,
    pub end_line: usize,
    pub field: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub range_sha256: String,
    pub claim_start_byte: usize,
    pub claim_end_byte: usize,
    pub claim_sha256: String,
    pub use_kind: ExplanationSourceUse,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplanationSourceUse {
    CurrentCode,
    BeforeCode,
    ObservedExecution,
}

pub fn explanation_text_hash(text: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(text.as_bytes()))
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
impl ExplanationRealization {
    /// Exhaustive generated-content traversal. All reader units use the same
    /// text/grounding/citation envelope; there are no unvalidated display strings.
    pub fn generated_units(&self) -> impl Iterator<Item = &ExplanationParagraph> {
        self.paragraphs.iter()
    }
    pub fn citations(&self) -> impl Iterator<Item = &ExplanationCitation> {
        self.generated_units().flat_map(|unit| &unit.citations)
    }
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
    pub semantic_approval_status: String,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplanationState {
    Current,
    Stale,
    Missing,
    Deleted,
    Withheld,
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
    /// Read-time permitted, citation-only bytes. Never stored in the realization.
    pub source_excerpts: Vec<ExplanationSourceExcerpt>,
    pub omitted_source_excerpt_count: usize,
}

/// Exact inspected range, bounded independently of prose and its full grounding.
/// A comparison describes observed file bytes, never an inferred patch.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExplanationSourceExcerpt {
    pub citation: ExplanationCitation,
    pub work_revision: Option<u64>,
    pub observed_at_unix_micros: i64,
    pub text: String,
    pub displayed_end_byte: usize,
    pub displayed_end_line: usize,
    pub omitted_utf8_bytes: usize,
    pub comparison: Option<String>,
    pub before_source_id: Option<String>,
    pub source_representation: Option<String>,
    pub observation_region: Option<Value>,
    pub execution: Option<Value>,
}
pub const EXPLANATION_SOURCE_EXCERPT_BYTE_LIMIT: usize = 16_384;
pub const EXPLANATION_SOURCE_RANGE_BYTE_LIMIT: usize = 4_096;

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
        ExplanationSubject::Work(work)
        | ExplanationSubject::WorkName(work)
        | ExplanationSubject::WorkSummary(work) => work,
        ExplanationSubject::Decision(decision) => {
            return prepare_decision(canonical, decision, language)
        }
        ExplanationSubject::Repository(_) => {
            return Err(
                "Repository explanation requires exact repository evidence from Operations".into(),
            )
        }
    };
    let index = crate::reading::WorkHistoryIndex::new(canonical);
    let history = index.0.get(&work).ok_or("Work not found in this Project")?;
    if matches!(
        subject,
        ExplanationSubject::WorkName(_) | ExplanationSubject::WorkSummary(_)
    ) {
        let evidence = short_work_evidence(
            history,
            matches!(subject, ExplanationSubject::WorkSummary(_)),
        );
        let question = if matches!(subject, ExplanationSubject::WorkName(_)) {
            "work_name"
        } else {
            "work_summary"
        };
        return finish_plan(canonical, subject, question, language, evidence);
    }
    let selected = history.materialize(canonical);
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
                "verification":s.verification.iter().map(|v|json!({"state":format!("{:?}",v.state),"source":v.source_id.map(|id|id.to_string()),"outcome":v.outcome,"scoped_run":v.scoped_run})).collect::<Vec<_>>(),
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
    for cp in &history.checkpoints {
        let basis = crate::reading::reading_basis(
            canonical,
            crate::ReadingRecord::Checkpoint(cp.id),
            cp.revision,
            "goal,kind,next_step,open_questions",
            cp.source_basis.clone(),
        );
        add(
            &format!("course:{}", cp.id),
            &basis,
            json!({
                "observed_at": cp.recorded_at.as_unix_micros(),
                "kind": format!("{:?}", cp.kind), "goal": cp.goal,
                "next_step": cp.next_step,
                "open_questions": cp.open_questions.iter().map(|q| json!({
                    "identity":q.question_id.to_string(), "revision":q.revision
                })).collect::<Vec<_>>(),
                "forgotten_source_count": canonical.forgotten_checkpoint_sources.iter()
                    .filter(|w| w.checkpoint_identity == cp.id.to_string()).count(),
            }),
        );
        if cp
            .state_change
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty())
        {
            let basis = crate::reading::reading_basis(
                canonical,
                crate::ReadingRecord::Checkpoint(cp.id),
                cp.revision,
                "state_change",
                cp.source_basis.clone(),
            );
            add(
                &format!("change:{}", cp.id),
                &basis,
                json!({"reported_change":cp.state_change,"observed_at":cp.recorded_at.as_unix_micros()}),
            );
        }
        if !cp.changed_paths.is_empty() || !cp.changed_source_basis.is_empty() {
            let basis = crate::reading::reading_basis(
                canonical,
                crate::ReadingRecord::Checkpoint(cp.id),
                cp.revision,
                "changed_paths,changed_source_basis",
                cp.changed_source_basis.clone(),
            );
            add(
                &format!("change_scope:{}", cp.id),
                &basis,
                json!({"paths":cp.changed_paths,
                "source_ids":cp.changed_source_basis.iter().map(ToString::to_string).collect::<Vec<_>>() }),
            );
        }
        let basis = crate::reading::reading_basis(
            canonical,
            crate::ReadingRecord::Checkpoint(cp.id),
            cp.revision,
            "known_limits,non_goals",
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
    for decision_id in &selected.decision_ids {
        let decision_plan = prepare_decision(canonical, *decision_id, language)?;
        evidence.extend(decision_plan.evidence.into_iter().map(|mut e| {
            e.key = format!("decision:{decision_id}:{}", e.key);
            e
        }));
    }
    finish_plan(canonical, subject, "work_outcome", language, evidence)
}

/// Select fields, not the article history. Every qualification has its own exact
/// revision/field/Source basis; command bodies are never needed to report facts.
fn short_work_evidence(
    history: &crate::reading::WorkHistory<'_>,
    summary: bool,
) -> Vec<ExplanationEvidence> {
    let goal = history.goal;
    let mut evidence = vec![ExplanationEvidence {
        key: "goal".into(),
        record_kind: "context_item".into(),
        identity: goal.id.to_string(),
        revision: goal.revision,
        field: "statement".into(),
        sources: goal.source_basis.iter().map(ToString::to_string).collect(),
        content: json!(goal.statement),
    }];
    if !summary {
        return evidence;
    }
    let mut add = |key: &str,
                   cp: &volicord_context::Checkpoint,
                   field: &str,
                   sources: Vec<volicord_context::SourceId>,
                   content: Value| {
        evidence.push(ExplanationEvidence {
            key: key.into(),
            record_kind: "checkpoint".into(),
            identity: cp.id.to_string(),
            revision: cp.revision,
            field: field.into(),
            sources: sources.iter().map(ToString::to_string).collect(),
            content,
        });
    };
    if let Some(report) = history.checkpoints.iter().rev().find(|cp| {
        cp.state_change
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty())
    }) {
        add(
            "result",
            report,
            "state_change",
            report.source_basis.clone(),
            json!(report.state_change),
        );
        if history
            .checkpoints
            .last()
            .is_some_and(|latest| latest.id != report.id)
            && (!report.known_limits.is_empty() || !report.non_goals.is_empty())
        {
            add(
                "result_limits",
                report,
                "known_limits,non_goals",
                report.source_basis.clone(),
                json!({"known_limits":report.known_limits,"non_goals":report.non_goals}),
            );
        }
    }
    if let Some(latest) = history.checkpoints.last() {
        add(
            "latest_state",
            latest,
            "work_state",
            latest.source_basis.clone(),
            json!({"observed_at":latest.recorded_at.as_unix_micros(),"work_state":format!("{:?}", latest.work_state)}),
        );
        add(
            "review",
            latest,
            "user_review",
            latest.user_review.source_id.into_iter().collect(),
            json!({"state":format!("{:?}", latest.user_review.state),"source_id":latest.user_review.source_id.map(|id|id.to_string())}),
        );
        add(
            "acceptance",
            latest,
            "user_acceptance",
            latest.user_acceptance.source_id.into_iter().collect(),
            json!({"state":format!("{:?}", latest.user_acceptance.state),"source_id":latest.user_acceptance.source_id.map(|id|id.to_string())}),
        );
        add(
            "limits",
            latest,
            "known_limits,non_goals",
            latest.source_basis.clone(),
            json!({"known_limits":latest.known_limits,"non_goals":latest.non_goals}),
        );
        add(
            "next_step",
            latest,
            "next_step",
            latest.source_basis.clone(),
            json!(latest.next_step),
        );
    }
    if let Some((position, verification)) = history
        .checkpoints
        .iter()
        .enumerate()
        .rev()
        .find(|(_, cp)| !cp.verification.is_empty())
    {
        let later_change = history.checkpoints[position + 1..]
            .iter()
            .rev()
            .find(|cp| crate::reading::has_reported_change(cp));
        add(
            "verification",
            verification,
            "verification",
            verification
                .verification
                .iter()
                .filter_map(|v| v.source_id)
                .collect(),
            json!({"observed_at":verification.recorded_at.as_unix_micros(),"facts":verification.verification.iter().map(|v|json!({"state":format!("{:?}",v.state),"source_id":v.source_id.map(|id|id.to_string()),"outcome":v.outcome,"scoped_run":v.scoped_run})).collect::<Vec<_>>(),
                "coverage":if later_change.is_some() {"historical_after_reported_change"} else {"no_later_reported_change_not_coverage_proof"}}),
        );
        if let Some(change) = later_change {
            add(
                "verification_coverage",
                change,
                "state_change,changed_paths,changed_source_basis",
                change
                    .source_basis
                    .iter()
                    .chain(&change.changed_source_basis)
                    .copied()
                    .collect(),
                json!({"reported_change":change.state_change,"paths":change.changed_paths,"source_ids":change.changed_source_basis}),
            );
        }
    }
    evidence
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
    let mut evidence: Vec<ExplanationEvidence> = [
        ("choice", "choice", decision.user_source_basis.clone(), json!({"choice":format!("{:?}",decision.choice),"chosen_alternative_key":decision.chosen_alternative_key,"state":format!("{:?}",decision.state)})),
        ("user_rationale", "user_rationale", decision.user_source_basis.clone(), json!(decision.user_rationale)),
        ("recommendation", "displayed_recommendation", decision.recommendation_source_basis.clone(), json!({"alternative_key":decision.recommended_alternative_key,"rationale":decision.recommendation_rationale})),
        ("consequences", "displayed_alternatives", decision.recommendation_source_basis.clone(), json!(decision.displayed_alternatives.iter().map(|a|json!({"key":a.key,"label":a.label,"consequence":a.consequence})).collect::<Vec<_>>())),
        ("applicability", "applicability", decision.source_basis.clone(), json!({"scope":format!("{:?}",decision.work_scope),"paths":lifecycle.decision.applicability.paths,"components":lifecycle.decision.applicability.components,"work_contexts":lifecycle.decision.applicability.work_contexts,"assumptions":decision.assumptions,"revisit_triggers":decision.revisit_triggers,"known_limits":decision.known_limits,"review_basis":decision.review_basis(crate::FixedLocale::English)})),
    ].into_iter().map(|(key,field,sources,content)| ExplanationEvidence {
        key:key.into(),field:field.into(),sources:sources.into_iter().map(|id|id.to_string()).collect(),content,
        record_kind:"decision".into(),identity:id.to_string(),revision:decision.revision,
    }).collect();
    let question = canonical
        .active_questions
        .iter()
        .chain(&canonical.terminal_question_history)
        .find(|q| {
            q.project_id == canonical.project.id
                && q.id == lifecycle.decision.question_id
                && q.revision == lifecycle.decision.question_revision
        });
    evidence.push(match question {
        Some(q) => ExplanationEvidence { key:"question_context".into(), record_kind:"question".into(),
            identity:q.id.to_string(), revision:q.revision,
            field:"prompt_basis,why_it_matters_now,established_facts,trade_offs,uncertainty,known_limits,what_the_answer_unlocks".into(),
            sources:q.source_basis.iter().chain(q.established_facts.iter().flat_map(|f| &f.source_basis))
                .map(ToString::to_string).collect(),
            content:json!({"availability":"Available","problem":q.prompt_basis,
                "why_it_matters_now":q.why_it_matters_now,"established_facts":q.established_facts,
                "trade_offs":q.trade_offs,"uncertainty":q.uncertainty,"known_limits":q.known_limits,
                "what_the_answer_unlocks":q.what_the_answer_unlocks}) },
        None => ExplanationEvidence { key:"question_context".into(), record_kind:"decision".into(),
            identity:id.to_string(), revision:decision.revision, field:"question_id,question_revision".into(),
            sources:Vec::new(), content:json!({"availability":"Unavailable",
                "question_id":lifecycle.decision.question_id.to_string(),
                "question_revision":lifecycle.decision.question_revision,
                "reason":"Exact originating Question revision is unavailable; do not reconstruct its problem or history"}) },
    });
    finish_plan(
        canonical,
        ExplanationSubject::Decision(id),
        "decision_rationale",
        language,
        evidence,
    )
}

/// Operations supplies the verified exact snapshot/binding scope. It is Derived
/// evidence, never a user-recorded Project Purpose or a fabricated Work.
pub fn prepare_repository_explanation(
    canonical: &CanonicalReadBasis,
    analysis: volicord_repository_intelligence::AnalysisSnapshotId,
    repository_source: volicord_context::SourceId,
    basis: Value,
    language: &str,
) -> Result<ExplanationPlan, String> {
    if language.trim().is_empty() || language.len() > 128 || language.chars().any(char::is_control)
    {
        return Err("a bounded nonempty requested language is required".into());
    }
    let mut evidence = vec![ExplanationEvidence {
        key: "repository_basis".into(),
        record_kind: "analysis_snapshot".into(),
        identity: analysis.to_string(),
        revision: 1,
        field: "repository_scope".into(),
        sources: vec![repository_source.to_string()],
        content: basis,
    }];
    for item in canonical
        .context_items
        .iter()
        .filter(|i| i.role == ContextItemRole::ProjectPurpose)
    {
        evidence.push(ExplanationEvidence {
            key: format!("project_purpose:{}", item.id),
            record_kind: "context_item".into(),
            identity: item.id.to_string(),
            revision: item.revision,
            field: "statement".into(),
            sources: item.source_basis.iter().map(ToString::to_string).collect(),
            content: json!(item.statement),
        });
    }
    finish_plan(
        canonical,
        ExplanationSubject::Repository(analysis),
        "repository_understanding",
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
    let source_status = sources.into_iter().map(|id| {
        match canonical.sources.iter().find(|s| s.source.id.to_string() == *id) {
            None => json!({"identity":id,"availability":"Unavailable","freshness":"Unknown",
                "body":"historical_body_not_retained"}),
            Some(s) => json!({"identity":s.source.id.to_string(),"immutable":true,
                "recorded_at":s.source.recorded_at.as_unix_micros(),
                "snapshot":s.snapshot_basis,"availability":format!("{:?}",s.availability),
                "freshness":format!("{:?}",s.freshness),"actor":format!("{:?}",s.source.actor),
                "observer":format!("{:?}",s.source.observer),
                "observation":match &s.source.payload {
                    volicord_context::SourcePayload::CurrentHostUserTurn { host, session, .. } =>
                        json!({"kind":"current_host_user_turn","host":host,"session":session,"body":"policy_withheld"}),
                    other => json!(format!("{other:?}")),
                },
                "body": if matches!(s.source.payload, volicord_context::SourcePayload::CurrentHostUserTurn { .. }) {
                    "policy_withheld"
                } else { "historical_body_not_retained" }}),
        }
    }).collect();
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
        ExplanationSubject::WorkName(_) => "Author exactly one heading naming this Work from its original Goal. Describe the requested problem/action, never claim an implementation or successful outcome. No technical Source is needed for a request-grounded name.",
        ExplanationSubject::WorkSummary(_) => "Author exactly one concise current-progress synopsis connecting the Goal to the actually reported approach/result and latest state. Use all selected field keys; summarize material qualifications rather than listing metadata. Without result, keep the report missing while describing supported progress. Preserve partial, failed, paused, abandoned and no-change outcomes. Review and acceptance are independent, including explicit resets; checks never imply completion or acceptance. Latest nonempty verification is a recorded fact, not current-code proof; verification_coverage marks a later meaningful change even with blank report. Preserve result_limits and current limits and next direction; a blank next_step has no older fallback. Canonical reports and checks establish reported meaning without command bodies. Claims about observed implementation/execution beyond these facts need their own permitted Source citations. Never invent an approach, completion or acceptance or extract article prose. This summary has independent admission and lifetime.",
        ExplanationSubject::Work(_) => "Author heading/synopsis and a task-specific article from goal, reported result/change:*, chronological course:* history, verification, next_step and limits:*. Reports establish no code mechanism. Read permitted callers/helpers/config/tests; paths alone prove no effect. Summarize command outcomes with citations; raw logs/JSON stay in Source inspection. Later reports never erase earlier failures or prove a retest.",
        ExplanationSubject::Repository(_) => "Explain the repository as a system across its material responsibilities, relationships and distinct significant operations, independently of Work. repository_basis supports scope, identity and limitations; technical meaning needs actual retrieved Source evidence. Inventory/structural coverage is not complete semantic understanding.",
        ExplanationSubject::Decision(_) => "Explain question_context, alternatives/consequences, recorded recommendation AND its rationale, actual user choice/delegation and only recorded user_rationale, scope/assumptions/revisit limits. Distinguish later interpretation. Missing technical Source limits verification, never erases recorded reasons. Selection proves no implementation or performance. Invent no user reason.",
    };
    let question_grounding = question_grounding(subject, &evidence);
    let mut plan = ExplanationPlan { project_id:canonical.project.id.to_string(), subject,
        question:question.into(), requested_language:language.into(), evidence, source_status, conflicts,
        instructions:format!("{questions} Every unit needs exact evidence_keys satisfying question_grounding; citations supplement keys. Adjacent comparison_condition/result each need grounding. Observation/comparison/flow_step/reading_point require citations. All generated units share validation and lifecycle. Reports, state, execution, review and acceptance differ; earlier checks cannot cover later changes. Use requested language, current format {EXPLANATION_VERSION}, exact fingerprint and self-reported generator (unknown model null). Product compose derives Source/claim UTF-8 coordinates and hashes. Host authority is external: missing data is neither denial nor consent. Preserve uncertainty; admission is not semantic approval. Return paragraphs question/text/evidence_keys/citations."), question_grounding, retention_budget:ExplanationRetentionBudget { response_byte_limit:EXPLANATION_BYTE_LIMIT,
            retained_byte_limit:EXPLANATION_RETAINED_BYTE_LIMIT, metadata_byte_reserve:0, response_byte_capacity:0 }, fingerprint:String::new() };
    if matches!(subject, ExplanationSubject::Repository(_)) {
        plan.instructions = format!("{questions} Every unit needs exact prepared evidence_keys. All roles are optional; use heading/synopsis/section_heading/purpose/observation/narrative/reading_point/limits in reader order. Every non-limits unit, including editorial headings, needs retrieved current_code citations; scope-only limits may cite repository_basis. Quiet heading actions in the reader do not waive admission or lifecycle. Preserve uncertainty and distinguish recorded Project Purpose from inference. Use the requested language and self-reported host/session/agent/model; unknown model stays null. Return the current ExplanationRealization format version {EXPLANATION_VERSION}, exact plan fingerprint/language and paragraphs. Product compose derives Source/claim UTF-8 coordinates and hashes from selected ranges; valid bindings are not entailment or semantic approval. No background dispatch or consent is created. {REPOSITORY_AUTHORING}");
    }
    if matches!(
        subject,
        ExplanationSubject::WorkName(_) | ExplanationSubject::WorkSummary(_)
    ) {
        plan.instructions = format!("{questions} Use the requested language, exact current fingerprint and format {EXPLANATION_VERSION}, and self-reported host/session/agent/model (unknown model null). Every unit needs the exact prepared evidence keys required above. Source/claim coordinates and hashes are bound by explicit compose from permitted selections. Never extract, relabel or rebind an earlier article or other publication. Record only this scope; sibling publications have separate provenance and validity. Structural admission establishes no semantic approval, verification or acceptance. Host authority is external; preparation grants no transmission or background permission. Ordinary reads never generate.");
    }
    if matches!(
        subject,
        ExplanationSubject::Work(_) | ExplanationSubject::Decision(_)
    ) {
        plan.instructions.push(' ');
        plan.instructions.push_str(ARTICLE_AUTHORING);
    }
    plan.retention_budget = explanation_retention_budget(&plan)?;
    // Admission includes the final fingerprint, even though the hash preimage
    // itself has an empty fingerprint. A plan exactly at the limit stays valid.
    let bytes = serde_json::to_vec(&plan).map_err(|e| e.to_string())?;
    let size = bytes.len() + "sha256:".len() + 64;
    if size > EXPLANATION_PLAN_BYTE_LIMIT {
        return Err(format!("explanation preparation JSON is {size} bytes; limit {EXPLANATION_PLAN_BYTE_LIMIT}; evidence cannot be represented without truncation; seek Product support for this evidence shape before generation"));
    }
    plan.fingerprint = format!("sha256:{:x}", Sha256::digest(bytes));
    Ok(plan)
}

fn question_grounding(
    subject: ExplanationSubject,
    evidence: &[ExplanationEvidence],
) -> Vec<ExplanationQuestionGrounding> {
    use ExplanationQuestion::*;
    match subject {
        ExplanationSubject::WorkName(_) => {
            return vec![ExplanationQuestionGrounding {
                question: Heading,
                required_any_evidence_keys: vec!["goal".into()],
            }]
        }
        ExplanationSubject::WorkSummary(_) => {
            return vec![ExplanationQuestionGrounding {
                question: Synopsis,
                required_any_evidence_keys: vec![if evidence.iter().any(|e| e.key == "result") {
                    "result"
                } else {
                    "goal"
                }
                .into()],
            }]
        }
        _ => (),
    }
    let present_or_goal = |key: &str| {
        if evidence.iter().any(|e| e.key == key) {
            key.to_owned()
        } else {
            "goal".into()
        }
    };
    let change_keys = || {
        std::iter::once(present_or_goal("result"))
            .chain(
                evidence
                    .iter()
                    .filter(|e| e.key.starts_with("change:"))
                    .map(|e| e.key.clone()),
            )
            .collect()
    };
    let rules = match subject {
        ExplanationSubject::Work(_) => vec![
            (Purpose, vec!["goal".into()]),
            (ReportedChange, change_keys()),
            (ExpectedEffect, change_keys()),
            (Verification, vec![present_or_goal("verification")]),
            (NextStep, vec!["next_step".into()]),
        ],
        ExplanationSubject::Repository(_) => vec![
            (Purpose, vec!["repository_basis".into()]),
            (Observation, vec!["repository_basis".into()]),
            (ReadingPoint, vec!["repository_basis".into()]),
        ],
        ExplanationSubject::Decision(_) => vec![
            (UserRationale, vec!["user_rationale".into()]),
            (Recommendation, vec!["recommendation".into()]),
            (Consequences, vec!["consequences".into()]),
            (Applicability, vec!["applicability".into()]),
        ],
        ExplanationSubject::WorkName(_) | ExplanationSubject::WorkSummary(_) => Vec::new(),
    };
    rules
        .into_iter()
        .chain([
            (Limits, Vec::new()),
            (Heading, Vec::new()),
            (Synopsis, Vec::new()),
            (SectionHeading, Vec::new()),
            (Narrative, Vec::new()),
            (ComparisonCondition, Vec::new()),
            (ComparisonResult, Vec::new()),
        ])
        .chain(
            matches!(subject, ExplanationSubject::Work(_))
                .then_some([
                    (Observation, Vec::new()),
                    (Comparison, Vec::new()),
                    (ReadingPoint, Vec::new()),
                    (FlowStep, Vec::new()),
                ])
                .into_iter()
                .flatten(),
        )
        .map(
            |(question, required_any_evidence_keys)| ExplanationQuestionGrounding {
                question,
                required_any_evidence_keys,
            },
        )
        .collect()
}

pub fn validate_explanation(
    plan: &ExplanationPlan,
    response: &ExplanationRealization,
) -> Result<(), String> {
    if response.format_kind != EXPLANATION_KIND || response.format_version != EXPLANATION_VERSION {
        return Err("unsupported explanation format; regenerate from current preparation".into());
    }
    if response.plan_fingerprint != plan.fingerprint || response.language != plan.requested_language
    {
        return Err("stale preparation or language mismatch; prepare again".into());
    }
    if response.generator.host.trim().is_empty() || response.generator.session.trim().is_empty() {
        return Err("active host and session provenance are required".into());
    }
    let response_bytes = serde_json::to_vec(response)
        .map_err(|e| e.to_string())?
        .len();
    if response_bytes > EXPLANATION_BYTE_LIMIT {
        return Err(format!("explanation compact realization JSON is {response_bytes} bytes; limit {EXPLANATION_BYTE_LIMIT}; reduce response prose or generator metadata, preserve useful reader content and evidence keys, then retry recording"));
    }
    let budget = explanation_retention_budget(plan)?;
    if plan.retention_budget != budget {
        return Err(
            "explanation retention budget does not match its evidence basis; prepare again".into(),
        );
    }
    if response_bytes > budget.response_byte_capacity {
        return Err(format!("explanation compact realization JSON is {response_bytes} bytes; prepared response capacity {}; retained limit {}; preserve grounding and seek Product support for this evidence shape", budget.response_byte_capacity, budget.retained_byte_limit));
    }
    // Use the same requirements for producer visibility and consumer admission.
    let grounding = question_grounding(plan.subject, &plan.evidence);
    if plan.question_grounding != grounding {
        return Err("question grounding does not match prepared evidence; prepare again".into());
    }
    if response.paragraphs.is_empty() || response.paragraphs.len() > 32 {
        return Err("a reader document requires 1..32 grounded units".into());
    }
    crate::article_sections(response.generated_units().map(|p| p.question))?;
    if matches!(
        plan.subject,
        ExplanationSubject::WorkName(_) | ExplanationSubject::WorkSummary(_)
    ) && (response.paragraphs.len() != 1
        || !response.paragraphs[0]
            .evidence_keys
            .iter()
            .any(|key| key == "goal"))
    {
        return Err(
            "Work name/summary requires one independently grounded unit including goal".into(),
        );
    }
    if matches!(plan.subject, ExplanationSubject::WorkName(_))
        && response.citations().next().is_some()
    {
        return Err(
            "Work name describes the Goal; technical claims belong in summary/article".into(),
        );
    }
    if matches!(plan.subject, ExplanationSubject::WorkSummary(_))
        && plan
            .evidence
            .iter()
            .any(|e| !response.paragraphs[0].evidence_keys.contains(&e.key))
    {
        return Err(
            "Work summary must ground Goal and every selected current-progress qualification"
                .into(),
        );
    }
    if response
        .paragraphs
        .iter()
        .any(|p| !grounding.iter().any(|r| r.question == p.question))
    {
        return Err("answer question does not belong to prepared subject".into());
    }
    for role in [ExplanationQuestion::Heading, ExplanationQuestion::Synopsis] {
        if response
            .generated_units()
            .filter(|p| p.question == role)
            .count()
            > 1
        {
            return Err("heading and synopsis must each be a single authored unit".into());
        }
    }
    for (index, p) in response.generated_units().enumerate() {
        // A comparison is a pair of individually grounded fragments, not a
        // nested untracked string or a table reconstructed by the renderer.
        if (p.question == ExplanationQuestion::ComparisonCondition
            && response.paragraphs.get(index + 1).map(|p| p.question)
                != Some(ExplanationQuestion::ComparisonResult))
            || (p.question == ExplanationQuestion::ComparisonResult
                && index
                    .checked_sub(1)
                    .and_then(|i| response.paragraphs.get(i))
                    .map(|p| p.question)
                    != Some(ExplanationQuestion::ComparisonCondition))
        {
            return Err(
                "comparison requires adjacent condition/result units with independent grounding"
                    .into(),
            );
        }
        if p.citations.len() > 32 {
            return Err("a reader unit supports at most 32 citations".into());
        }
        let limit = match p.question {
            ExplanationQuestion::Heading | ExplanationQuestion::SectionHeading => 240,
            ExplanationQuestion::Synopsis => 2_000,
            _ => EXPLANATION_PARAGRAPH_CHARACTER_LIMIT,
        };
        if (limit != EXPLANATION_PARAGRAPH_CHARACTER_LIMIT && p.text.chars().count() > limit)
            || p.text.contains('\0')
        {
            return Err(format!("reader unit {:?} exceeds {limit} characters or contains NUL; author a bounded unit without clipping grounding", p.question));
        }
        if matches!(
            p.question,
            ExplanationQuestion::Observation
                | ExplanationQuestion::Comparison
                | ExplanationQuestion::ReadingPoint
                | ExplanationQuestion::FlowStep
        ) && p.citations.is_empty()
        {
            return Err(
                "observations, comparisons and reading points require exact Source citations"
                    .into(),
            );
        }
        if matches!(plan.subject, ExplanationSubject::Repository(_))
            && p.question != ExplanationQuestion::Limits
            && p.citations.is_empty()
        {
            return Err("repository reader units require exact Source citations".into());
        }
        for citation in &p.citations {
            let claim = p
                .text
                .get(citation.claim_start_byte..citation.claim_end_byte)
                .filter(|s| !s.trim().is_empty())
                .ok_or("invalid citation claim range")?;
            if explanation_text_hash(claim) != citation.claim_sha256
                || citation.start_byte >= citation.end_byte
                || citation.source_revision == 0
                || citation.start_line == 0
                || citation.end_line < citation.start_line
            {
                return Err("invalid citation claim binding or Source range".into());
            }
            if matches!(plan.subject, ExplanationSubject::Decision(_)) {
                return Err("Work Source citations require a Work subject".into());
            }
        }
        if p.text.chars().count() > EXPLANATION_PARAGRAPH_CHARACTER_LIMIT {
            return Err(format!("explanation paragraph exceeds {EXPLANATION_PARAGRAPH_CHARACTER_LIMIT} characters; split the prose while preserving question and grounding"));
        }
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
        let rule = grounding
            .iter()
            .find(|r| r.question == p.question)
            .ok_or("answer question does not belong to prepared subject")?;
        if !rule.required_any_evidence_keys.is_empty()
            && !p
                .evidence_keys
                .iter()
                .any(|key| rule.required_any_evidence_keys.contains(key))
        {
            return Err(format!(
                "answer {:?} must reference one of {}",
                p.question,
                rule.required_any_evidence_keys.join(", ")
            ));
        }
    }
    Ok(())
}

/// One retained representation for preparation budgeting, recording and freshness.
/// Original evidence bodies and Source observations stay canonical; all retained
/// grounding fields remain intact.
pub fn retain_explanation(
    plan: &ExplanationPlan,
    realization: ExplanationRealization,
    generated_at_unix_micros: i64,
) -> RetainedExplanation {
    let mut evidence = plan.evidence.clone();
    for e in &mut evidence {
        if e.record_kind != "analysis_snapshot" {
            e.content = Value::Null;
        }
    }
    let mut source_status = plan.source_status.clone();
    for status in &mut source_status {
        if let Some(object) = status.as_object_mut() {
            object.remove("observation");
        }
    }
    RetainedExplanation {
        realization,
        project_id: plan.project_id.clone(),
        subject: plan.subject,
        question: plan.question.clone(),
        evidence,
        source_status,
        conflicts: plan.conflicts.clone(),
        generated_at_unix_micros,
        generator_identity_status: GENERATOR_IDENTITY_STATUS.into(),
        semantic_approval_status: "not_established".into(),
    }
}

fn explanation_retention_budget(
    plan: &ExplanationPlan,
) -> Result<ExplanationRetentionBudget, String> {
    let placeholder = ExplanationRealization {
        format_kind: String::new(),
        format_version: EXPLANATION_VERSION,
        plan_fingerprint: String::new(),
        language: String::new(),
        generator: ExplanationGenerator {
            host: String::new(),
            session: String::new(),
            agent: None,
            model: None,
        },
        paragraphs: Vec::new(),
    };
    let response_bytes = serde_json::to_vec(&placeholder)
        .map_err(|e| e.to_string())?
        .len();
    let retained = retain_explanation(plan, placeholder, i64::MIN);
    let metadata_byte_reserve = serde_json::to_vec(&retained)
        .map_err(|e| e.to_string())?
        .len()
        - response_bytes;
    let required = metadata_byte_reserve + EXPLANATION_MIN_RESPONSE_BYTE_CAPACITY;
    if required > EXPLANATION_RETAINED_BYTE_LIMIT {
        return Err(format!("explanation retained envelope needs {required} bytes ({metadata_byte_reserve} metadata plus {EXPLANATION_MIN_RESPONSE_BYTE_CAPACITY} guaranteed response); limit {EXPLANATION_RETAINED_BYTE_LIMIT}; this evidence shape cannot support generation; seek Product support, do not drop grounding or repeatedly shorten the answer"));
    }
    Ok(ExplanationRetentionBudget {
        response_byte_limit: EXPLANATION_BYTE_LIMIT,
        retained_byte_limit: EXPLANATION_RETAINED_BYTE_LIMIT,
        metadata_byte_reserve,
        response_byte_capacity: EXPLANATION_BYTE_LIMIT
            .min(EXPLANATION_RETAINED_BYTE_LIMIT - metadata_byte_reserve),
    })
}

pub fn encode_retained_explanation(retained: &RetainedExplanation) -> Result<String, String> {
    let content = serde_json::to_string(retained).map_err(|e| e.to_string())?;
    if content.len() > EXPLANATION_RETAINED_BYTE_LIMIT {
        return Err(format!("explanation retained envelope is {} bytes; limit {EXPLANATION_RETAINED_BYTE_LIMIT}; no recording was published; prepare again or seek Product support for this evidence shape",content.len()));
    }
    Ok(content)
}
