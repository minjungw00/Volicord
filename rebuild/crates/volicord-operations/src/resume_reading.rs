//! Purpose-specific Recall serialization. Full reader payloads stay in their
//! existing readers; canonical resumption meaning is never paid for after them.
use serde_json::{json, Value};
use volicord_context::{ProjectId, VerificationFact};
use volicord_projections::{BriefDecision, FixedLocale, QuestionAnswers, UnderstandingWork};

fn inspection(kind: &str, identity: &str, revision: u64, field: &str) -> Value {
    json!({"tool":"canonical_inspect","record_kind":kind,"record_id":identity,
        "revision":revision,"field":field,"offset":0})
}

pub(crate) fn verification_json(
    fact: &VerificationFact,
    checkpoint: &str,
    revision: u64,
    index: usize,
) -> Value {
    json!({"state":crate::recall::verification_state_name(fact.state),
        "source_id":fact.source_id.map(|id|id.to_string()),"outcome_recorded":fact.outcome.is_some(),
        "scoped_run_recorded":fact.scoped_run.is_some(),"verification_index":index,
        "detail_inspection":inspection("checkpoint",checkpoint,revision,"verification")})
}

#[derive(Clone, Copy)]
enum StateDimension {
    Work,
    Verification,
    Review,
    Acceptance,
}

fn state_json(
    state: &volicord_projections::WorkStateObservation,
    dimension: StateDimension,
) -> Value {
    let field = match dimension {
        StateDimension::Work => "work_state",
        StateDimension::Verification => "verification",
        StateDimension::Review => "user_review",
        StateDimension::Acceptance => "user_acceptance",
    };
    let mut value = json!({"checkpoint_id":state.checkpoint_id.to_string(),"checkpoint_revision":state.checkpoint_revision,
        "observed_at_unix_micros":state.observed_at.as_unix_micros(),
        "later_changed_checkpoint_ids":state.later_changed_checkpoint_ids.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "detail_inspection":inspection("checkpoint",&state.checkpoint_id.to_string(),state.checkpoint_revision,field)});
    let fact = match dimension {
        StateDimension::Work => {
            value["work_source_basis"] = json!(state
                .work_source_basis
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>());
            json!(crate::recall::work_state_name(state.work_state))
        }
        StateDimension::Verification => json!(state
            .verification
            .iter()
            .enumerate()
            .map(|(index, fact)| verification_json(
                fact,
                &state.checkpoint_id.to_string(),
                state.checkpoint_revision,
                index
            ))
            .collect::<Vec<_>>()),
        StateDimension::Review => {
            json!({"state":crate::recall::user_review_state_name(state.user_review.state),"source_id":state.user_review.source_id.map(|id|id.to_string())})
        }
        StateDimension::Acceptance => {
            json!({"state":crate::recall::user_acceptance_state_name(state.user_acceptance.state),"source_id":state.user_acceptance.source_id.map(|id|id.to_string())})
        }
    };
    value[field] = fact;
    value
}

fn text_json(text: &volicord_projections::ReadingText) -> Value {
    let mut value = crate::recall::reading_text_json(text);
    if let Some(object) = value.as_object_mut() {
        // Keep a recorded quotation once in its original language, with the
        // unchanged exact revision/field and whole-field expansion basis.
        object.remove("display_english");
        object.remove("display_korean");
    }
    value
}

fn reading_reference(
    answers: Option<&QuestionAnswers>,
    project: ProjectId,
    kind: &str,
    identity: &str,
    language: &str,
) -> Value {
    let provenance = answers.and_then(|a| a.provenance.as_ref());
    json!({"role":"generated_interpretation",
        "explanation_state":answers.map_or(volicord_projections::ExplanationState::Missing,|a|a.explanation_state),
        "diagnostic":answers.and_then(|a|a.diagnostic.as_ref()),
        "publication_id":provenance.and_then(|p|p.publication_id.as_ref()),
        "plan_fingerprint":provenance.map(|p|&p.fingerprint),
        "inspect":{"tool":"explanation_read","arguments":{"project_id":project.to_string(),
            "subject_kind":kind,"subject_id":identity,"language":language,"action":"page","offset":0}}})
}

pub(crate) fn work_json(
    work: &UnderstandingWork,
    project: ProjectId,
    language: &str,
    locale: FixedLocale,
) -> Value {
    let answers = volicord_projections::work_answers(work, language, locale);
    let id = work.work_item_id.to_string();
    let report = &work.reading.verification_report;
    let verification_keys = work
        .reading
        .answers
        .verification
        .iter()
        .flat_map(|state| {
            (0..state.verification.len()).map(move |index| {
                format!(
                    "checkpoint:{}@{}:verification[{index}]",
                    state.checkpoint_id, state.checkpoint_revision
                )
            })
        })
        .collect::<std::collections::BTreeSet<_>>();
    let status_facts = answers
        .facts
        .iter()
        .filter(|fact| match fact.question.as_str() {
            "WorkState"
            | "UserReview"
            | "UserAcceptance"
            | "VerificationReportSummary"
            | "VerificationCoverage"
            | "HistoricalAdversity"
            | "AdverseObservation" => true,
            "VerificationState" => {
                !fact.evidence_keys.is_empty()
                    && fact
                        .evidence_keys
                        .iter()
                        .all(|key| verification_keys.contains(key))
            }
            _ => false,
        })
        .collect::<Vec<_>>();
    use volicord_projections::ReportedCheckState::*;
    json!({"reading_profile":"resume","work_item_id":id,
        "state":crate::recall::understanding_work_state_name(work.state),
        "reading_title":answers.card_name(),"reading_summary":answers.card_summary(),
        "status_facts":status_facts,
        "next_action":answers.next_step_answer(),
        "reading_references":{
            "name":reading_reference(answers.work_name.as_deref(),project,"work_name",&id,language),
            "summary":reading_reference(answers.work_summary.as_deref(),project,"work_summary",&id,language),
            "article":reading_reference(Some(&answers),project,"work",&id,language)},
        "creation_event":work.creation_event,"latest_work_record_event":work.latest_work_record_event,
        "changed_paths":work.changed_paths,"changed_components":work.changed_components,
        "checkpoint_ids":work.checkpoint_ids.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "decision_ids":work.decision_ids.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "open_question_ids":work.open_question_ids.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "source_ids":work.source_basis.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "known_limits":work.reading.known_limits,"non_goals":work.reading.non_goals,
        "evidence":{"goal":text_json(&work.reading.goal),
            "result":work.reading.answers.result.as_ref().map(text_json),
            "result_observed_at":work.reading.answers.result_observed_at.map(|t|t.as_unix_micros()),
            "next_step":text_json(&work.reading.next_step),
            "latest_state":work.reading.answers.latest_state.as_ref().map(|s|state_json(s,StateDimension::Work)),
            "verification":work.reading.answers.verification.as_ref().map(|s|state_json(s,StateDimension::Verification)),
            "review":work.reading.answers.review.as_ref().map(|s|state_json(s,StateDimension::Review)),
            "acceptance":work.reading.answers.acceptance.as_ref().map(|s|state_json(s,StateDimension::Acceptance)),
            "code_availability":format!("{:?}",work.reading.code_availability).to_lowercase(),
            "analysis_snapshot_ids":work.reading.analysis_snapshot_basis,
            "repository_snapshot_ids":work.reading.repository_snapshot_basis,
            "source_status":work.reading.evidence_source_status.iter().map(|s|json!({
                "source_id":s.source_id.to_string(),"availability":s.availability.map(|a|format!("{a:?}").to_lowercase()),
                "freshness":format!("{:?}",s.freshness).to_lowercase()})).collect::<Vec<_>>()},
        "verification_report":{"scope":"resume_counts","report_count_unit":"checkpoint_verification_report",
            "report_count_scope":"selected_work_checkpoint_history","reported_test_count_basis":"original_report_text_only",
            "report_counts":{"total":report.checks.len(),"passed":report.report_count(Passed),"failed":report.report_count(Failed),
                "partial":report.report_count(Partial),"not_run":report.report_count(NotRun)},
            "execution_count":report.executions.len(),
            "inspect":{"interface":"local_cli","arguments":["--project",project.to_string().as_str(),"work","verification","--work",id.as_str()]},
            "limit":"Historical report counts do not establish test totals, resolution, or coverage of the latest changes; inspect the full report."}})
}

pub(crate) fn decision_json(
    decision: &BriefDecision,
    project: ProjectId,
    language: &str,
    locale: FixedLocale,
) -> Value {
    let mut value = crate::recall::decision_record_json(decision, locale);
    if let Some(object) = value.as_object_mut() {
        // Keep the same canonical choice, alternatives, separate reasons and
        // applicability; do not spend their budget on the reader's article.
        object.insert("reading_profile".into(), json!("resume"));
        let answers = volicord_projections::decision_answers(decision, language, locale);
        object.insert(
            "reading_reference".into(),
            reading_reference(
                Some(&answers),
                project,
                "decision",
                &decision.decision_id.to_string(),
                language,
            ),
        );
    }
    value
}
