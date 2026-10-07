use serde_json::{json, Value};
use volicord_context::{
    CheckpointKind, ContextItemRole, DecisionWorkScope, VerificationState, WorkState,
};
use volicord_projections::{BriefContextItem, BriefDecisionState, OmissionReason, ResumeBrief};

/// Shared CLI/host representation of the bounded Resume Brief. Source metadata
/// remains inspectable without copying raw user turns or repository content.
pub fn resume_brief_json(
    brief: &ResumeBrief,
    language: &str,
    locale: volicord_projections::FixedLocale,
) -> Value {
    let checkpoint = brief.latest_meaningful_checkpoint.as_ref().map(|value| json!({
            "identity":value.id.to_string(),
            "work_item_id":value.work_item_id.map(|identity| identity.to_string()),
            "revision":value.revision,
            "kind":checkpoint_kind_name(value.kind),
            "goal":value.goal,
            "work_state":work_state_name(value.work_state),
            "state_change":value.state_change,
            "source_basis":value.source_basis.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
            "changed_source_basis":value.changed_source_basis.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
            "changed_paths":value.changed_paths,
            "applied_decisions":value.applied_decisions.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
            "verification":value.verification.iter().map(|fact| json!({"state":verification_state_name(fact.state),"source_id":fact.source_id.map(|id| id.to_string()),"outcome":fact.outcome})).collect::<Vec<_>>(),
            "user_review":{"state":user_review_state_name(value.user_review.state),"source_id":value.user_review.source_id.map(|id| id.to_string())},
            "user_acceptance":{"state":user_acceptance_state_name(value.user_acceptance.state),"source_id":value.user_acceptance.source_id.map(|id| id.to_string())},
            "known_limits":value.known_limits,
            "non_goals":value.non_goals,
            "open_questions":value.open_questions.iter().map(|question| json!({"identity":question.question_id.to_string(),"revision":question.revision})).collect::<Vec<_>>(),
            "next_step":value.next_step,
            "handoff_to":value.handoff_to,
            "recorded_at_unix_micros":value.recorded_at.as_unix_micros(),
        }));
    let output = json!({
        "selected_work":brief.selected_work.as_ref().map(|w|work_reading_json(w,language,locale)),
        "project_id":brief.project_id.to_string(), "project_name":brief.project_name,
        "project_purpose":brief.project_purpose.iter().map(context_json).collect::<Vec<_>>(),
        "goals":brief.goals_and_why.iter().map(|item| &item.statement).collect::<Vec<_>>(),
        "goal_basis":brief.goals_and_why.iter().map(context_json).collect::<Vec<_>>(),
        "behaviorally_relevant_context":brief.behaviorally_relevant_context.iter().map(context_json).collect::<Vec<_>>(),
        "active_decision_count":brief.decisions.iter().filter(|item| item.state != BriefDecisionState::Superseded).count(),
        "decisions":brief.decisions.iter().map(|d| decision_reading_json(d,language,locale)).collect::<Vec<_>>(),
        "checkpoint":checkpoint,
        "open_questions":brief.open_questions.iter().map(|item| json!({
            "identity":item.question_id.to_string(), "revision":item.revision, "prompt":item.prompt,
            "on_current_frontier":item.on_current_frontier, "blocked_basis":item.blocked_basis,
            "what_the_answer_unlocks":item.what_the_answer_unlocks,
            "source_basis":item.source_basis.iter().map(ToString::to_string).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "risks_assumptions_and_limits":brief.risks_assumptions_and_limits.iter().map(context_json).collect::<Vec<_>>(),
        "declared_assumptions":brief.declared_assumptions, "known_limits":brief.known_limits,
        "next_step":brief.selected_work.as_ref().and_then(|w|volicord_projections::work_answers(w,language,locale).recorded_next_action().map(|a|a.recorded_text.clone())),
        "used_sources":brief.used_sources.iter().map(|item| item.source.id.to_string()).collect::<Vec<_>>(),
        "source_details":brief.used_sources.iter().map(|item| json!({
            "identity":item.source.id.to_string(), "actor":item.source.actor,
            "observer":item.source.observer, "snapshot_basis":item.snapshot_basis,
            "availability":format!("{:?}",item.availability).to_lowercase(),
            "freshness":format!("{:?}",item.freshness).to_lowercase(),
        })).collect::<Vec<_>>(),
        "snapshots":brief.snapshots.iter().map(|item| json!({
            "analysis_snapshot":item.analysis_snapshot, "repository_snapshot":item.repository_snapshot,
            "freshness":item.freshness, "capabilities":item.capabilities,
        })).collect::<Vec<_>>(),
        "omissions":brief.omissions.iter().map(|item| json!({
            "identity":item.identity, "kind":item.kind,
            "reason":match item.reason {
                OmissionReason::Bound => "bound", OmissionReason::Scope => "scope",
                OmissionReason::SupersededHistory => "superseded_history",
                OmissionReason::UnavailableBasis => "unavailable_basis",
                OmissionReason::FailedBasis => "failed_basis",
            },
            "expandable_basis":item.expandable_basis,
        })).collect::<Vec<_>>(),
        "omitted_count":brief.omitted_count,
        "proposals":brief.proposals.iter().map(|item| json!({
            "kind":item.kind, "basis":item.basis,
            "source_ids":item.source_ids.iter().map(ToString::to_string).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "read_only":true,
        "transport_budget":{
            "mcp_result_bytes":crate::HOST_READ_RESULT_BYTE_BUDGET,
            "resume_brief_bytes":56 * 1024,
            "omission_rule":"Complete fields or stable suffix items only; transport_omission is not semantic content",
            "inspection":{"canonical":"canonical_inspect", "analysis":"repository_understanding", "scope":"same project and returned record/snapshot identities", "canonical_manifest":{"tool":"canonical_inspect","offset":0}, "detail_fields":"Use record_kind, record_id, revision and field; join compact_json_utf8 chunks and continue with next_offset and expected_fingerprint"}
        },
    });
    crate::bounded_read_section(output, 56 * 1024)
}

fn decision_work_scope_json(scope: DecisionWorkScope) -> Value {
    match scope {
        DecisionWorkScope::Unresolved => json!({"kind":"unresolved"}),
        DecisionWorkScope::ProjectWide => json!({"kind":"project_wide"}),
        DecisionWorkScope::WorkItem(identity) => {
            json!({"kind":"work_item","work_item_id":identity.to_string()})
        }
    }
}

fn context_json(item: &BriefContextItem) -> Value {
    json!({"identity":item.identity.to_string(), "revision":item.revision, "role":context_item_role_name(item.role),
        "statement":item.statement,
        "source_ids":item.source_basis.iter().map(ToString::to_string).collect::<Vec<_>>()})
}

const fn context_item_role_name(role: ContextItemRole) -> &'static str {
    match role {
        ContextItemRole::ProjectPurpose => "project_purpose",
        ContextItemRole::Goal => "goal",
        ContextItemRole::Fact => "fact",
        ContextItemRole::Assumption => "assumption",
        ContextItemRole::Constraint => "constraint",
        ContextItemRole::Preference => "preference",
        ContextItemRole::Risk => "risk",
        ContextItemRole::Learning => "learning",
        ContextItemRole::KnownLimit => "known_limit",
    }
}

const fn checkpoint_kind_name(value: CheckpointKind) -> &'static str {
    match value {
        CheckpointKind::Completion => "completion",
        CheckpointKind::Pause => "pause",
        CheckpointKind::Handoff => "handoff",
    }
}

pub(crate) const fn work_state_name(value: WorkState) -> &'static str {
    match value {
        WorkState::InProgress => "in_progress",
        WorkState::Paused => "paused",
        WorkState::Completed => "completed",
        WorkState::Abandoned => "abandoned",
        WorkState::Superseded => "superseded",
    }
}

pub(crate) const fn verification_state_name(value: VerificationState) -> &'static str {
    match value {
        VerificationState::NotRun => "not_run",
        VerificationState::Partial => "partial",
        VerificationState::Passed => "passed",
        VerificationState::Failed => "failed",
    }
}

pub(crate) const fn user_review_state_name(
    value: volicord_context::UserReviewState,
) -> &'static str {
    match value {
        volicord_context::UserReviewState::NotRequested => "not_requested",
        volicord_context::UserReviewState::Pending => "pending",
        volicord_context::UserReviewState::Reviewed => "reviewed",
    }
}

pub(crate) const fn user_acceptance_state_name(
    value: volicord_context::UserAcceptanceState,
) -> &'static str {
    match value {
        volicord_context::UserAcceptanceState::NotRequested => "not_requested",
        volicord_context::UserAcceptanceState::Pending => "pending",
        volicord_context::UserAcceptanceState::Accepted => "accepted",
        volicord_context::UserAcceptanceState::Rejected => "rejected",
    }
}

/// Shared ordinary answers and explicit quotation/audit evidence for CLI and MCP.
pub fn work_reading_json(
    work: &volicord_projections::UnderstandingWork,
    language: &str,
    locale: volicord_projections::FixedLocale,
) -> Value {
    json!({"work_item_id":work.work_item_id.to_string(),"state":understanding_work_state_name(work.state),
        "answers":volicord_projections::work_answers(work,language,locale),
        "changed_paths":work.changed_paths,"changed_components":work.changed_components,"checkpoint_ids":work.checkpoint_ids.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "decision_ids":work.decision_ids.iter().map(ToString::to_string).collect::<Vec<_>>(),"open_question_ids":work.open_question_ids.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "source_ids":work.source_basis.iter().map(ToString::to_string).collect::<Vec<_>>(),"evidence":{"source_status":work.reading.evidence_source_status.iter().map(|s|json!({"source_id":s.source_id.to_string(),"availability":s.availability.map(debug_name),"freshness":debug_name(s.freshness)})).collect::<Vec<_>>(),"goal":reading_text_json(&work.reading.goal),"result":work.reading.answers.result.as_ref().map(reading_text_json),
            "result_observed_at":work.reading.answers.result_observed_at.map(|t|t.as_unix_micros()),"original_changes":work.reading.changes.iter().map(reading_text_json).collect::<Vec<_>>(),
            "next_step":reading_text_json(&work.reading.next_step),"states":work.reading.states.iter().map(state_observation_json).collect::<Vec<_>>(),
            "latest_state":work.reading.answers.latest_state.as_ref().map(state_observation_json),"verification":work.reading.answers.verification.as_ref().map(state_observation_json),
            "review":work.reading.answers.review.as_ref().map(state_observation_json),"acceptance":work.reading.answers.acceptance.as_ref().map(state_observation_json),
            "code_availability":debug_name(work.reading.code_availability),"analysis_snapshot_ids":work.reading.analysis_snapshot_basis,"repository_snapshot_ids":work.reading.repository_snapshot_basis}})
}

pub(crate) fn reading_text_json(text: &volicord_projections::ReadingText) -> Value {
    let representation = match text.representation {
        volicord_projections::ReadingRepresentation::OriginalQuotation => "original_quotation",
        volicord_projections::ReadingRepresentation::Excerpt => "excerpt",
        volicord_projections::ReadingRepresentation::Unavailable => "unavailable",
    };
    let (kind, identity) = match text.basis.record {
        volicord_projections::ReadingRecord::ContextItem(id) => ("context_item", id.to_string()),
        volicord_projections::ReadingRecord::Checkpoint(id) => ("checkpoint", id.to_string()),
        volicord_projections::ReadingRecord::Decision(id) => ("decision", id.to_string()),
    };
    json!({"original_text":text.original_text,"display_english":text.display_english,"display_korean":text.display_korean,
        "representation":representation,"availability":debug_name(text.availability),
        "original_language_preserved":text.original_language_preserved,
        "omitted_utf8_bytes":text.omitted_utf8_bytes,"omitted_characters":text.omitted_characters,"gaps":text.gaps,
        "detail_inspection":{"tool":"canonical_inspect","record_kind":kind,"record_id":identity,"revision":text.basis.revision,"field":text.basis.field},
        "basis":{"record_kind":kind,"identity":identity,"revision":text.basis.revision,
            "available_revisions":text.basis.available_revisions,"field":text.basis.field,
            "source_ids":text.basis.source_basis.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "source_status":text.basis.source_status.iter().map(|source| json!({"source_id":source.source_id.to_string(),"availability":source.availability.map(debug_name),"freshness":debug_name(source.freshness),"snapshot_basis":source.snapshot_basis,"actor":source.actor,"observer":source.observer,"recorded_at_unix_micros":source.recorded_at.map(|time| time.as_unix_micros())})).collect::<Vec<_>>(),
            "analysis_snapshot_ids":text.basis.analysis_snapshot_basis,"repository_snapshot_ids":text.basis.repository_snapshot_basis}})
}

fn state_observation_json(state: &volicord_projections::WorkStateObservation) -> Value {
    json!({
        "checkpoint_id":state.checkpoint_id.to_string(), "checkpoint_revision":state.checkpoint_revision,
        "detail_inspection":{"tool":"canonical_inspect","record_kind":"checkpoint","record_id":state.checkpoint_id.to_string(),"revision":state.checkpoint_revision,"field":"verification"},
        "observed_at_unix_micros":state.observed_at.as_unix_micros(),
        "work_state":work_state_name(state.work_state), "work_source_basis":state.work_source_basis.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "verification":state.verification.iter().map(|fact| json!({"state":verification_state_name(fact.state),"source_id":fact.source_id.map(|id| id.to_string()),"outcome":fact.outcome})).collect::<Vec<_>>(),
        "user_review":{"state":user_review_state_name(state.user_review.state),"source_id":state.user_review.source_id.map(|id| id.to_string())},
        "user_acceptance":{"state":user_acceptance_state_name(state.user_acceptance.state),"source_id":state.user_acceptance.source_id.map(|id| id.to_string())},
        "later_changed_checkpoint_ids":state.later_changed_checkpoint_ids.iter().map(ToString::to_string).collect::<Vec<_>>()
    })
}

fn debug_name(value: impl std::fmt::Debug) -> String {
    format!("{value:?}").to_lowercase()
}

const fn understanding_work_state_name(
    state: volicord_projections::UnderstandingWorkState,
) -> &'static str {
    use volicord_projections::UnderstandingWorkState::*;
    match state {
        Open => "open",
        InProgress => "in_progress",
        Paused => "paused",
        Completed => "completed",
        Abandoned => "abandoned",
        Superseded => "superseded",
    }
}
pub fn decision_reading_json(
    d: &volicord_projections::BriefDecision,
    language: &str,
    locale: volicord_projections::FixedLocale,
) -> Value {
    json!({"identity":d.decision_id.to_string(),"revision":d.revision,
        "state":match d.state {BriefDecisionState::Current=>"current",BriefDecisionState::StaleBasis=>"stale_basis",BriefDecisionState::ReviewRequired=>"review_required",BriefDecisionState::Superseded=>"superseded",BriefDecisionState::UnavailableBasis=>"unavailable_basis"},
        "answers":volicord_projections::decision_answers(d,language,locale),"work_scope":decision_work_scope_json(d.work_scope),
        "choice":format!("{:?}",d.choice),"chosen_alternative_key":d.chosen_alternative_key,"recommended_alternative_key":d.recommended_alternative_key,
        "displayed_alternatives":d.displayed_alternatives.iter().map(|a|json!({"alternative_key":a.key,"label":a.label,"expected_consequence":a.consequence})).collect::<Vec<_>>(),
        "source_basis":d.source_basis.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "evidence":{"user_rationale":d.user_rationale,"recommendation_rationale":d.recommendation_rationale,
            "user_source_basis":d.user_source_basis.iter().map(ToString::to_string).collect::<Vec<_>>(),"recommendation_source_basis":d.recommendation_source_basis.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "available_revisions":d.available_revisions,"assumptions":d.assumptions,"revisit_triggers":d.revisit_triggers,
            "question_reference":{"identity":d.question_reference.question_id.to_string(),"revision":d.question_reference.revision},
            "question_context":d.question_context.as_ref().map(|q|json!({"identity":q.question_id.to_string(),"revision":q.revision,"prompt":q.prompt,"source_ids":q.source_basis.iter().map(ToString::to_string).collect::<Vec<_>>()})),
            "question_source_status":d.question_source_status.iter().map(|s|json!({"identity":s.source_id.to_string(),"availability":s.availability.map(|a|format!("{a:?}").to_lowercase()),"freshness":format!("{:?}",s.freshness).to_lowercase()})).collect::<Vec<_>>(),
            "question_uncertainty":d.question_uncertainty,"known_limits":d.known_limits,"review_basis":d.review_basis(locale)}})
}

/// Identity-only navigation under the named Candidate Inspection authority.
/// Never attach Candidate prose to canonical Work answers or document claims.
pub fn work_learning_inspection_json(
    projection: &volicord_projections::ProjectProjection,
) -> Value {
    let references = projection.candidate_inspection.iter().filter_map(|candidate| {
        let learning = candidate.learning_deliberation.as_ref()?;
        let work = learning.goal_context_id;
        let known_work = projection.selected_work.iter().chain(&projection.work_history)
            .chain(&projection.work_overview.current.items).chain(&projection.work_overview.completed.items)
            .chain(&projection.work_overview.remaining.items).chain(&projection.work_overview.next_steps.items)
            .any(|w| w.work_item_id == work);
        known_work.then(|| json!({"work_item_id":work.to_string(),"candidate_id":candidate.candidate_id.to_string(),
            "revision":candidate.revision,"canonical_decision":false,
            "inspect":{"tool":"candidate_inspect","project_id":projection.overview.project_id.to_string(),
                "candidate_id":candidate.candidate_id.to_string(),"revision":candidate.revision,"field":"learning_deliberation"}}))
    }).collect::<Vec<_>>();
    json!({"dependency":format!("{:?}", projection.candidate_dependency).to_lowercase(),
        "learning_references":crate::bounded_read_section(json!(references), 8 * 1024),
        "omitted_candidate_count":projection.issues.iter().filter(|i| i.affected_scope == "candidate_inspection")
            .map(|i| i.omitted_count).sum::<usize>(),
        "authority":"candidate_inspection_only"})
}
