//! Exact retained evidence reads. No source locator is dereferenced here.
use crate::{bounded_read_section, Error, LocalOperations};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_context::{CanonicalReadBasis, ContextItemId, DecisionWorkScope, ProjectId, SourceId};
use volicord_inquiry::CandidateId;
use volicord_projections::{inspect_candidate, CandidateContentAccess};

const CHUNK_BYTES: usize = 2048;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceDetailRequest {
    pub record_kind: String,
    pub record_id: String,
    pub revision: u64,
    pub field: String,
    #[serde(default)]
    pub offset: usize,
    pub expected_fingerprint: Option<String>,
    pub work_item_id: Option<ContextItemId>,
}

impl LocalOperations {
    /// Page a complete canonical manifest, independently of projection list bounds.
    /// Continuation refuses a changed read basis rather than shifting offsets.
    pub fn canonical_inspection_page(
        &self,
        project: ProjectId,
        offset: usize,
        expected_fingerprint: Option<&str>,
    ) -> Result<Value, Error> {
        let _inspection = self.layout().acquire_health_lock()?;
        let basis = self.canonical_basis(project)?;
        let records =
            volicord_projections::build_canonical_inspection(&basis, usize::MAX, &mut Vec::new());
        let fingerprint = digest(&format!("{basis:?}"));
        check_continuation(offset, expected_fingerprint, &fingerprint)?;
        if offset > records.len() {
            return Err(Error::new(
                "inspection offset exceeds retained record count",
            ));
        }
        let end = offset.saturating_add(8).min(records.len());
        let page = records[offset..end].iter().map(|record| {
            let kind = match record.kind {
                volicord_projections::CanonicalInspectionKind::ContextItem => "context_item".to_owned(),
                kind => format!("{kind:?}").to_lowercase(),
            };
            json!({"kind":kind,"identity":record.identity,"revision":record.revision,
                "lifecycle_state":record.lifecycle_state,"statement_role":record.statement_role,
                "summary":bounded_read_section(json!(record.summary),1024),
                "source_basis":bounded_read_section(json!(record.source_basis.iter().map(ToString::to_string).collect::<Vec<_>>()),1024),
                "detail_fields":canonical_fields(&kind),
                "inspect":{"tool":"canonical_inspect","project_id":project.to_string(),"record_kind":kind,"record_id":record.identity,"revision":record.revision}})
        }).collect::<Vec<_>>();
        Ok(
            json!({"project_id":project.to_string(),"records":page,"offset":offset,
            "total_count":records.len(),"omitted_count":records.len()-end,
            "fingerprint":fingerprint,"next_offset":(end<records.len()).then_some(end),"read_only":true}),
        )
    }

    pub fn canonical_evidence_detail(
        &self,
        project: ProjectId,
        request: &EvidenceDetailRequest,
    ) -> Result<Value, Error> {
        validate_request(request)?;
        let _inspection = self.layout().acquire_health_lock()?;
        let basis = self.canonical_basis(project)?;
        let identity = &request.record_id;
        let mut revision = None;
        let mut work = None;
        let mut sources = Vec::new();
        let mut fields = json!({});
        match request.record_kind.as_str() {
            "context_item" => {
                if let Some(item) = basis
                    .context_items
                    .iter()
                    .find(|item| item.id.to_string() == *identity)
                {
                    revision = Some(item.revision);
                    work =
                        (item.role == volicord_context::ContextItemRole::Goal).then_some(item.id);
                    sources = item.source_basis.clone();
                    fields = json!({"statement":item.statement,"source_basis":ids(&sources),"applicability":item.applicability});
                }
            }
            "checkpoint" => {
                if let Some(cp) = basis
                    .checkpoint_history
                    .iter()
                    .chain(basis.latest_checkpoint.iter())
                    .find(|cp| cp.id.to_string() == *identity)
                {
                    revision = Some(cp.revision);
                    work = cp.work_item_id;
                    sources = cp
                        .source_basis
                        .iter()
                        .chain(&cp.changed_source_basis)
                        .copied()
                        .chain(cp.verification.iter().filter_map(|v| v.source_id))
                        .chain(cp.user_review.source_id)
                        .chain(cp.user_acceptance.source_id)
                        .collect();
                    fields = json!({"goal":cp.goal,"state_change":cp.state_change,"next_step":cp.next_step,
                    "changed_paths":cp.changed_paths,"source_basis":ids(&cp.source_basis),"changed_source_basis":ids(&cp.changed_source_basis),
                    "applied_decisions":cp.applied_decisions.iter().map(ToString::to_string).collect::<Vec<_>>(),
                    "verification":cp.verification.iter().map(|v|json!({"state":crate::recall::verification_state_name(v.state),"source_id":v.source_id.map(|s|s.to_string()),"outcome":v.outcome})).collect::<Vec<_>>(),
                    "user_review":{"state":crate::recall::user_review_state_name(cp.user_review.state),"source_id":cp.user_review.source_id.map(|s|s.to_string())},
                    "user_acceptance":{"state":crate::recall::user_acceptance_state_name(cp.user_acceptance.state),"source_id":cp.user_acceptance.source_id.map(|s|s.to_string())},
                    "known_limits":cp.known_limits,"non_goals":cp.non_goals,"handoff_to":cp.handoff_to,
                    "open_questions":cp.open_questions.iter().map(|q|json!({"identity":q.question_id.to_string(),"revision":q.revision})).collect::<Vec<_>>()});
                }
            }
            "decision" => {
                if let Some(d) = basis
                    .active_decisions
                    .iter()
                    .chain(&basis.superseded_decisions)
                    .map(|d| &d.decision)
                    .find(|d| d.id.to_string() == *identity)
                {
                    revision = Some(d.revision);
                    work = match d.work_scope {
                        DecisionWorkScope::WorkItem(id) => Some(id),
                        _ => None,
                    };
                    sources = d.displayed_recommendation.source_basis.clone();
                    sources.push(d.user_turn_source_id);
                    fields = json!({"user_rationale":d.user_rationale,"displayed_recommendation.rationale":d.displayed_recommendation.rationale,
                    "assumptions":d.assumptions,"revisit_triggers":d.revisit_triggers,"source_basis":ids(&sources),"applicability":d.applicability,
                    "displayed_alternatives":d.displayed_alternatives.iter().map(|a|json!({"alternative_key":a.key,"label":a.label,"expected_consequence":a.consequence})).collect::<Vec<_>>()});
                }
            }
            "question" => {
                if let Some(q) = basis
                    .active_questions
                    .iter()
                    .chain(&basis.terminal_question_history)
                    .find(|q| q.id.to_string() == *identity)
                {
                    revision = Some(q.revision);
                    sources = q.source_basis.clone();
                    fields = json!({"prompt_basis":q.prompt_basis,"source_basis":ids(&sources),"assumptions":q.assumptions,
                    "known_limits":q.known_limits,"trade_offs":q.trade_offs,"uncertainty":q.uncertainty,
                    "why_it_matters_now":q.why_it_matters_now,"what_the_answer_unlocks":q.what_the_answer_unlocks});
                }
            }
            "source" => {
                if let Some(s) = basis
                    .sources
                    .iter()
                    .find(|s| s.source.id.to_string() == *identity)
                {
                    revision = basis
                        .revisions
                        .iter()
                        .find(|r| {
                            r.record_identity == *identity
                                && r.record_kind == volicord_context::CanonicalRecordKind::Source
                        })
                        .and_then(|r| r.revisions.last().copied());
                    sources.push(s.source.id);
                    // Metadata only. In particular CurrentHostUserTurn.turn is not a body reader.
                    fields = json!({"observation":source_observation(s)});
                }
            }
            _ => return Err(Error::new("unsupported canonical evidence record kind")),
        }
        if !canonical_fields(&request.record_kind).contains(&request.field.as_str()) {
            return Err(Error::new("unsupported canonical evidence field"));
        }
        let metadata = json!({"work_item_id":work.map(|id|id.to_string()),"source_status":source_status(&basis,&sources),
            "privacy":"retained_canonical_fields_only_no_source_body_access"});
        if revision != Some(request.revision) {
            return Ok(unavailable(
                project,
                request,
                if revision.is_some() {
                    "revision_unavailable"
                } else {
                    "record_unavailable"
                },
                metadata,
            ));
        }
        if request.work_item_id.is_some_and(|id| Some(id) != work) {
            return Err(Error::new(
                "evidence record does not belong to the requested Work",
            ));
        }
        let value = fields.get(&request.field).cloned().unwrap_or(Value::Null);
        if value.is_null() {
            return Ok(unavailable(
                project,
                request,
                if request.record_kind == "source" && request.field == "body" {
                    "historical_body_not_retained"
                } else {
                    "field_not_recorded"
                },
                metadata,
            ));
        }
        detail_page(project, request, value, metadata)
    }

    pub fn candidate_evidence_detail(
        &self,
        project: ProjectId,
        candidate: CandidateId,
        request: &EvidenceDetailRequest,
    ) -> Result<Value, Error> {
        validate_request(request)?;
        if request.record_kind != "candidate" || request.record_id != candidate.to_string() {
            return Err(Error::new("Candidate evidence identity mismatch"));
        }
        let _inspection = self.layout().acquire_health_lock()?;
        let _canonical = self.canonical_basis(project)?;
        let basis = self.candidate_basis(project)?;
        let inspection = inspect_candidate(
            &basis,
            candidate,
            CandidateContentAccess::AllowBoundedSummary,
            volicord_context::Clock::now(&mut volicord_context::SystemClock)
                .map_err(|e| Error::with_source("inspection clock unavailable", e))?,
        );
        let metadata = json!({"content_omission":inspection.content_omission.as_ref().map(|o|format!("{o:?}")),
            "privacy":"candidate_inspection_retention_and_forgetting_policy"});
        if inspection.revision != Some(request.revision) {
            return Ok(unavailable(
                project,
                request,
                if inspection.exists {
                    "revision_unavailable"
                } else {
                    "record_unavailable"
                },
                metadata,
            ));
        }
        let fields = json!({"summary":inspection.bounded_summary,"engineering_choice_discovery":inspection.engineering_choice_discovery,
            "materiality_review":inspection.materiality_review,"learning_deliberation":inspection.learning_deliberation,
            "repository_research_basis":inspection.repository_research_basis});
        if ![
            "summary",
            "engineering_choice_discovery",
            "materiality_review",
            "learning_deliberation",
            "repository_research_basis",
        ]
        .contains(&request.field.as_str())
        {
            return Err(Error::new("unsupported Candidate evidence field"));
        }
        if inspection.content_omission.is_some() {
            return Ok(unavailable(project, request, "content_withheld", metadata));
        }
        if request.work_item_id.is_some() {
            return Err(Error::new(
                "Candidate detail uses its retained Goal scope; Work selector is unsupported",
            ));
        }
        let value = fields[&request.field].clone();
        if value.is_null() {
            return Ok(unavailable(
                project,
                request,
                "field_not_recorded",
                metadata,
            ));
        }
        detail_page(project, request, value, metadata)
    }
}

fn canonical_fields(kind: &str) -> &'static [&'static str] {
    match kind {
        "context_item" => &["statement", "source_basis", "applicability"],
        "checkpoint" => &[
            "goal",
            "state_change",
            "next_step",
            "verification",
            "changed_paths",
            "source_basis",
            "changed_source_basis",
            "applied_decisions",
            "user_review",
            "user_acceptance",
            "known_limits",
            "non_goals",
            "open_questions",
            "handoff_to",
        ],
        "decision" => &[
            "user_rationale",
            "displayed_recommendation.rationale",
            "displayed_alternatives",
            "assumptions",
            "revisit_triggers",
            "source_basis",
            "applicability",
        ],
        "question" => &[
            "prompt_basis",
            "source_basis",
            "assumptions",
            "known_limits",
            "trade_offs",
            "uncertainty",
            "why_it_matters_now",
            "what_the_answer_unlocks",
        ],
        "source" => &["observation", "body"],
        _ => &[],
    }
}
fn ids(sources: &[SourceId]) -> Vec<String> {
    sources.iter().map(ToString::to_string).collect()
}
fn source_status(basis: &CanonicalReadBasis, sources: &[SourceId]) -> Value {
    json!(sources.iter().map(|id|{
        let source=basis.sources.iter().find(|s|s.source.id==*id);
        json!({"source_id":id.to_string(),"availability":source.map(|s|format!("{:?}",s.availability).to_lowercase()),
            "freshness":source.map(|s|format!("{:?}",s.freshness).to_lowercase()),"snapshot_basis":source.and_then(|s|s.snapshot_basis.as_ref())})
    }).collect::<Vec<_>>())
}
fn source_observation(s: &volicord_context::SourceReadBasis) -> Value {
    use volicord_context::SourcePayload::*;
    let payload = match &s.source.payload {
        CommandExecution {
            command_label,
            invocation_fingerprint,
            outcome,
        } => {
            json!({"kind":"command_execution","command_label":command_label,"invocation_fingerprint":invocation_fingerprint,"exit_code":outcome.exit_code,"termination":format!("{:?}",outcome.termination).to_lowercase(),"body_retention":"not_retained"})
        }
        CurrentHostUserTurn { host, session, .. } => {
            json!({"kind":"current_host_user_turn","host":host,"session":session,"body_access":"policy_withheld"})
        }
        _ => {
            json!({"kind":"source_reference","snapshot_basis":s.snapshot_basis,"body_retention":"not_retained"})
        }
    };
    json!({"payload":payload,"availability":format!("{:?}",s.availability).to_lowercase(),"freshness":format!("{:?}",s.freshness).to_lowercase(),"recorded_at_unix_micros":s.source.recorded_at.as_unix_micros()})
}
fn validate_request(r: &EvidenceDetailRequest) -> Result<(), Error> {
    if r.record_id.len() != 32
        || !r
            .record_id
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        || r.revision == 0
        || r.field.len() > 128
    {
        return Err(Error::new(
            "invalid exact evidence identity, revision or field",
        ));
    }
    Ok(())
}
fn check_continuation(offset: usize, expected: Option<&str>, actual: &str) -> Result<(), Error> {
    if offset > 0 && expected.is_none() {
        return Err(Error::new("continuation requires expected_fingerprint"));
    }
    if expected.is_some_and(|value| value != actual) {
        return Err(Error::new(
            "evidence read basis changed; continuation rejected",
        ));
    }
    Ok(())
}
fn digest(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
fn unavailable(
    project: ProjectId,
    r: &EvidenceDetailRequest,
    reason: &str,
    metadata: Value,
) -> Value {
    json!({"project_id":project.to_string(),"record_kind":r.record_kind,"record_id":r.record_id,"revision":r.revision,"field":r.field,
        "state":"unavailable","reason":reason,"metadata":bounded_read_section(metadata,4096),"read_only":true,"next_offset":null})
}
fn detail_page(
    project: ProjectId,
    r: &EvidenceDetailRequest,
    value: Value,
    metadata: Value,
) -> Result<Value, Error> {
    let serialized = value.to_string();
    let fingerprint=digest(&json!({"project":project.to_string(),"kind":r.record_kind,"id":r.record_id,"revision":r.revision,"field":r.field,"value":value,"metadata":metadata}).to_string());
    check_continuation(r.offset, r.expected_fingerprint.as_deref(), &fingerprint)?;
    if r.offset > serialized.len() || !serialized.is_char_boundary(r.offset) {
        return Err(Error::new("invalid evidence UTF-8 byte offset"));
    }
    let mut end = r.offset.saturating_add(CHUNK_BYTES).min(serialized.len());
    while !serialized.is_char_boundary(end) {
        end -= 1;
    }
    Ok(
        json!({"project_id":project.to_string(),"record_kind":r.record_kind,"record_id":r.record_id,"revision":r.revision,"field":r.field,
        "state":if end<serialized.len(){"partial"}else{"complete"},"encoding":"compact_json_utf8","chunk":&serialized[r.offset..end],
        "offset":r.offset,"next_offset":(end<serialized.len()).then_some(end),"total_utf8_bytes":serialized.len(),"remaining_utf8_bytes":serialized.len()-end,
        "fingerprint":fingerprint,"metadata":bounded_read_section(metadata,4096),"read_only":true}),
    )
}
