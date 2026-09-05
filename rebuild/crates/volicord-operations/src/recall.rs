use serde_json::{json, Value};
use volicord_context::{CheckpointKind, ContextItemRole, VerificationState, WorkState};
use volicord_projections::{BriefContextItem, BriefDecisionState, OmissionReason, ResumeBrief};

/// Shared CLI/host representation of the bounded Resume Brief. Source metadata
/// remains inspectable without copying raw user turns or repository content.
pub fn resume_brief_json(brief: &ResumeBrief) -> Value {
    let checkpoint = brief.latest_meaningful_checkpoint.as_ref().map(|value| json!({
            "identity":value.id.to_string(),
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
    json!({
        "project_id":brief.project_id.to_string(), "project_name":brief.project_name,
        "goals":brief.goals_and_why.iter().map(|item| &item.statement).collect::<Vec<_>>(),
        "goal_basis":brief.goals_and_why.iter().map(context_json).collect::<Vec<_>>(),
        "behaviorally_relevant_context":brief.behaviorally_relevant_context.iter().map(context_json).collect::<Vec<_>>(),
        "active_decision_count":brief.decisions.iter().filter(|item| item.state != BriefDecisionState::Superseded).count(),
        "decisions":brief.decisions.iter().map(|item| json!({
            "identity":item.decision_id.to_string(), "revision":item.revision,
            "state":match item.state {
                BriefDecisionState::Current => "current",
                BriefDecisionState::StaleBasis => "stale_basis",
                BriefDecisionState::ReviewRequired => "review_required",
                BriefDecisionState::Superseded => "superseded",
                BriefDecisionState::UnavailableBasis => "unavailable_basis",
            },
            "choice":format!("{:?}",item.choice), "rationale":item.user_rationale,
            "recommendation_rationale":item.recommendation_rationale,
            "assumptions":item.assumptions, "revisit_triggers":item.revisit_triggers,
            "source_basis":item.source_basis.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "question_uncertainty":item.question_uncertainty, "known_limits":item.known_limits,
            "expected_consequences":item.expected_consequences, "review_basis":item.review_basis,
        })).collect::<Vec<_>>(),
        "checkpoint":checkpoint,
        "open_questions":brief.open_questions.iter().map(|item| json!({
            "identity":item.question_id.to_string(), "revision":item.revision, "prompt":item.prompt,
            "on_current_frontier":item.on_current_frontier, "blocked_basis":item.blocked_basis,
            "what_the_answer_unlocks":item.what_the_answer_unlocks,
            "source_basis":item.source_basis.iter().map(ToString::to_string).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "risks_assumptions_and_limits":brief.risks_assumptions_and_limits.iter().map(context_json).collect::<Vec<_>>(),
        "declared_assumptions":brief.declared_assumptions, "known_limits":brief.known_limits,
        "next_step":brief.next_meaningful_step,
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
    })
}

fn context_json(item: &BriefContextItem) -> Value {
    json!({"identity":item.identity.to_string(), "role":context_item_role_name(item.role),
        "statement":item.statement,
        "source_ids":item.source_basis.iter().map(ToString::to_string).collect::<Vec<_>>()})
}

const fn context_item_role_name(role: ContextItemRole) -> &'static str {
    match role {
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

const fn work_state_name(value: WorkState) -> &'static str {
    match value {
        WorkState::InProgress => "in_progress",
        WorkState::Paused => "paused",
        WorkState::Completed => "completed",
        WorkState::Abandoned => "abandoned",
        WorkState::Superseded => "superseded",
    }
}

const fn verification_state_name(value: VerificationState) -> &'static str {
    match value {
        VerificationState::NotRun => "not_run",
        VerificationState::Partial => "partial",
        VerificationState::Passed => "passed",
        VerificationState::Failed => "failed",
    }
}

const fn user_review_state_name(value: volicord_context::UserReviewState) -> &'static str {
    match value {
        volicord_context::UserReviewState::NotRequested => "not_requested",
        volicord_context::UserReviewState::Pending => "pending",
        volicord_context::UserReviewState::Reviewed => "reviewed",
    }
}

const fn user_acceptance_state_name(value: volicord_context::UserAcceptanceState) -> &'static str {
    match value {
        volicord_context::UserAcceptanceState::NotRequested => "not_requested",
        volicord_context::UserAcceptanceState::Pending => "pending",
        volicord_context::UserAcceptanceState::Accepted => "accepted",
        volicord_context::UserAcceptanceState::Rejected => "rejected",
    }
}
