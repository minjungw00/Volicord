use std::{collections::BTreeSet, path::Path};

use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use volicord_store::{
    bootstrap::ProjectRecord,
    guards::{
        insert_unrecorded_change, list_expected_writes_matched_by_post_event,
        list_pending_expected_writes, mark_expected_write_matched, unrecorded_change,
        ExpectedWriteMatch, ExpectedWriteRecord, UnrecordedChangeInsert,
    },
};
use volicord_types::GuardDecision;

use super::GuardPhaseResult;
use crate::guard_command::{
    context::{guard_state_summary, ActiveWriteTicketSummary, GuardStateSummary},
    envelope::{event_time_or_now, GuardEnvelope},
    json_error,
    render::{context_json, tool_observation_json},
    stable_id,
    tool_observation::{tool_observation, ToolObservation},
    write_ticket::{
        active_write_ticket_match, normalized_observed_paths, ActiveWriteTicketMatchOutcome,
    },
    GuardCommandError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct PostToolCorrelation {
    matched_expected_writes: Vec<Value>,
    ticket_backed_observations: Vec<Value>,
    unrecorded_changes: Vec<Value>,
}

struct UnrecordedChangeContext<'a> {
    runtime_home: &'a Path,
    project: &'a ProjectRecord,
    envelope: &'a GuardEnvelope,
    summary: &'a GuardStateSummary,
    observation: &'a ToolObservation,
    changed: Vec<String>,
    correlation_status: &'static str,
    candidate_expected_write_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ExpectedWriteMatchOutcome {
    Matched(ExpectedWriteRecord),
    AlreadyMatched(ExpectedWriteRecord),
    NoCandidates,
    OutOfScope(Vec<String>),
    Ambiguous(Vec<String>),
}

pub(in crate::guard_command) fn handle_post_tool(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    input: &crate::guard_command::args::GuardInput,
) -> Result<GuardPhaseResult, GuardCommandError> {
    let summary = guard_state_summary(runtime_home, project, envelope, input)?;
    let observation = tool_observation(&input.raw_value, &project.repo_root);
    let correlation =
        record_post_tool_correlation(runtime_home, project, envelope, &summary, &observation)?;
    let decision = if correlation.unrecorded_changes.is_empty() {
        GuardDecision::Allow
    } else {
        GuardDecision::Warn
    };
    Ok(GuardPhaseResult::new(
        decision,
        json!({
            "decision": decision.as_str(),
            "allowed": true,
            "tool": tool_observation_json(&observation),
            "matched_expected_writes": correlation.matched_expected_writes,
            "ticket_backed_observations": correlation.ticket_backed_observations,
            "unrecorded_changes": correlation.unrecorded_changes,
            "context": context_json(&summary),
            "enforcement_level": "cooperative_detective"
        }),
    ))
}

fn record_post_tool_correlation(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    summary: &GuardStateSummary,
    observation: &ToolObservation,
) -> Result<PostToolCorrelation, GuardCommandError> {
    if observation.tool_name.as_deref() == Some("volicord.record_run") {
        return Ok(PostToolCorrelation {
            matched_expected_writes: Vec::new(),
            ticket_backed_observations: Vec::new(),
            unrecorded_changes: Vec::new(),
        });
    }
    let changed = normalized_observed_paths(observation.changed_paths.iter());
    if changed.is_empty() {
        return Ok(PostToolCorrelation {
            matched_expected_writes: Vec::new(),
            ticket_backed_observations: Vec::new(),
            unrecorded_changes: Vec::new(),
        });
    }
    let match_outcome =
        match_expected_write(runtime_home, project, envelope, observation, &changed)?;
    match match_outcome {
        ExpectedWriteMatchOutcome::Matched(record) => {
            mark_expected_write_matched(
                runtime_home,
                &project.project_id,
                &record.expected_write_id,
                ExpectedWriteMatch {
                    matched_post_tool_guard_event_id: envelope.event_id.clone(),
                    matched_paths_json: serde_json::to_string(&changed).map_err(json_error)?,
                    matched_at: envelope.occurred_at.clone(),
                },
            )?;
            Ok(PostToolCorrelation {
                matched_expected_writes: vec![matched_expected_write_json(&record, &changed)],
                ticket_backed_observations: Vec::new(),
                unrecorded_changes: Vec::new(),
            })
        }
        ExpectedWriteMatchOutcome::AlreadyMatched(record) => Ok(PostToolCorrelation {
            matched_expected_writes: vec![matched_expected_write_json(&record, &changed)],
            ticket_backed_observations: Vec::new(),
            unrecorded_changes: Vec::new(),
        }),
        ExpectedWriteMatchOutcome::NoCandidates => {
            match active_write_ticket_match(summary, &changed) {
                ActiveWriteTicketMatchOutcome::Matched(ticket) => Ok(PostToolCorrelation {
                    matched_expected_writes: Vec::new(),
                    ticket_backed_observations: vec![ticket_backed_observation_json(
                        &ticket, &changed,
                    )],
                    unrecorded_changes: Vec::new(),
                }),
                ActiveWriteTicketMatchOutcome::NoActiveTickets => Ok(PostToolCorrelation {
                    matched_expected_writes: Vec::new(),
                    ticket_backed_observations: Vec::new(),
                    unrecorded_changes: record_unrecorded_changes(UnrecordedChangeContext {
                        runtime_home,
                        project,
                        envelope,
                        summary,
                        observation,
                        changed,
                        correlation_status: "unmatched_expected_write",
                        candidate_expected_write_ids: Vec::new(),
                    })?,
                }),
                ActiveWriteTicketMatchOutcome::OutOfScope(ticket_ids) => Ok(PostToolCorrelation {
                    matched_expected_writes: Vec::new(),
                    ticket_backed_observations: Vec::new(),
                    unrecorded_changes: record_unrecorded_changes(UnrecordedChangeContext {
                        runtime_home,
                        project,
                        envelope,
                        summary,
                        observation,
                        changed,
                        correlation_status: "out_of_scope_write_ticket",
                        candidate_expected_write_ids: ticket_ids,
                    })?,
                }),
                ActiveWriteTicketMatchOutcome::Ambiguous(ticket_ids) => Ok(PostToolCorrelation {
                    matched_expected_writes: Vec::new(),
                    ticket_backed_observations: Vec::new(),
                    unrecorded_changes: record_unrecorded_changes(UnrecordedChangeContext {
                        runtime_home,
                        project,
                        envelope,
                        summary,
                        observation,
                        changed,
                        correlation_status: "ambiguous_write_ticket",
                        candidate_expected_write_ids: ticket_ids,
                    })?,
                }),
            }
        }
        ExpectedWriteMatchOutcome::OutOfScope(candidate_ids) => Ok(PostToolCorrelation {
            matched_expected_writes: Vec::new(),
            ticket_backed_observations: Vec::new(),
            unrecorded_changes: record_unrecorded_changes(UnrecordedChangeContext {
                runtime_home,
                project,
                envelope,
                summary,
                observation,
                changed,
                correlation_status: "out_of_scope_expected_write",
                candidate_expected_write_ids: candidate_ids,
            })?,
        }),
        ExpectedWriteMatchOutcome::Ambiguous(candidate_ids) => Ok(PostToolCorrelation {
            matched_expected_writes: Vec::new(),
            ticket_backed_observations: Vec::new(),
            unrecorded_changes: record_unrecorded_changes(UnrecordedChangeContext {
                runtime_home,
                project,
                envelope,
                summary,
                observation,
                changed,
                correlation_status: "ambiguous_expected_write",
                candidate_expected_write_ids: candidate_ids,
            })?,
        }),
    }
}

fn record_unrecorded_changes(
    context: UnrecordedChangeContext<'_>,
) -> Result<Vec<Value>, GuardCommandError> {
    if context.changed.is_empty() {
        return Ok(Vec::new());
    }
    let change_id = stable_id(
        "unrecorded_change",
        &[
            &context.envelope.event_id,
            &context.project.project_id,
            &context.changed.join("|"),
        ],
    );
    if unrecorded_change(
        context.runtime_home,
        &context.project.project_id,
        &change_id,
    )?
    .is_some()
    {
        return Ok(vec![json!({
            "unrecorded_change_id": change_id,
            "status": "already_recorded",
            "observed_paths": context.changed
        })]);
    }
    insert_unrecorded_change(
        context.runtime_home,
        &context.project.project_id,
        UnrecordedChangeInsert {
            unrecorded_change_id: change_id.clone(),
            session_id: context.envelope.session_id.clone(),
            connection_internal_id: context.envelope.connection_id.clone(),
            task_id: context.summary.active_task_id.clone(),
            summary: "Product file changes were observed after a host tool without a matching Volicord run record".to_owned(),
            observed_paths_json: serde_json::to_string(&context.changed).map_err(json_error)?,
            detection_json: json!({
                "source": "volicord_guard_post_tool",
                "tool_name": context.observation.tool_name,
                "exit_code": context.observation.exit_code,
                "success": context.observation.success,
                "status": context.observation.status,
                "correlation_status": context.correlation_status,
                "candidate_expected_write_ids": context.candidate_expected_write_ids,
                "active_write_ticket_ids": context.summary.active_write_tickets
                    .iter()
                    .map(|ticket| ticket.write_ticket_id.clone())
                    .collect::<Vec<_>>(),
                "ticket_scope_violation": matches!(
                    context.correlation_status,
                    "out_of_scope_expected_write" | "out_of_scope_write_ticket"
                ),
                "detector_role": "detective",
                "does_not_prevent_writes": true,
                "does_not_identify_actor": true
            })
            .to_string(),
            detected_at: context.envelope.occurred_at.clone(),
            metadata_json: json!({
                "guard_event_id": context.envelope.event_id
            })
            .to_string(),
        },
    )?;
    Ok(vec![json!({
        "unrecorded_change_id": change_id,
        "status": "unresolved",
        "observed_paths": context.changed
    })])
}

fn match_expected_write(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    observation: &ToolObservation,
    changed: &[String],
) -> Result<ExpectedWriteMatchOutcome, GuardCommandError> {
    let already_matched = list_expected_writes_matched_by_post_event(
        runtime_home,
        &project.project_id,
        &envelope.connection_id,
        &envelope.event_id,
    )?;
    let changed_set = changed.iter().cloned().collect::<BTreeSet<_>>();
    let already_matched = already_matched
        .into_iter()
        .filter(|record| expected_write_session_matches(record, envelope))
        .filter(|record| matched_paths_cover_observed(record, &changed_set))
        .collect::<Vec<_>>();
    if already_matched.len() == 1 {
        return Ok(ExpectedWriteMatchOutcome::AlreadyMatched(
            already_matched.into_iter().next().expect("length checked"),
        ));
    }
    if already_matched.len() > 1 {
        return Ok(ExpectedWriteMatchOutcome::Ambiguous(
            already_matched
                .into_iter()
                .map(|record| record.expected_write_id)
                .collect(),
        ));
    }

    let host_invocation_id = host_invocation_id_from_observation(observation);
    let observed_at = event_time_or_now(&envelope.occurred_at);
    let pending =
        list_pending_expected_writes(runtime_home, &project.project_id, &envelope.connection_id)?;
    let time_scoped = pending
        .into_iter()
        .filter(|record| expected_write_time_contains(record, observed_at))
        .collect::<Vec<_>>();

    let candidates = if let Some(host_id) = host_invocation_id.as_deref() {
        let exact = time_scoped
            .iter()
            .filter(|record| record.host_invocation_id.as_deref() == Some(host_id))
            .filter(|record| expected_write_session_matches(record, envelope))
            .cloned()
            .collect::<Vec<_>>();
        if exact.is_empty() {
            fallback_expected_write_candidates(&time_scoped, envelope, true)
        } else {
            exact
        }
    } else {
        fallback_expected_write_candidates(&time_scoped, envelope, false)
    };
    if candidates.is_empty() {
        return Ok(ExpectedWriteMatchOutcome::NoCandidates);
    }

    let path_matched = candidates
        .iter()
        .filter(|record| expected_paths_cover_observed(record, &changed_set))
        .cloned()
        .collect::<Vec<_>>();
    if path_matched.len() == 1 {
        return Ok(ExpectedWriteMatchOutcome::Matched(
            path_matched.into_iter().next().expect("length checked"),
        ));
    }
    if path_matched.len() > 1 {
        return Ok(ExpectedWriteMatchOutcome::Ambiguous(
            path_matched
                .into_iter()
                .map(|record| record.expected_write_id)
                .collect(),
        ));
    }
    let candidate_ids = candidates
        .into_iter()
        .map(|record| record.expected_write_id)
        .collect::<Vec<_>>();
    if candidate_ids.len() == 1 {
        Ok(ExpectedWriteMatchOutcome::OutOfScope(candidate_ids))
    } else {
        Ok(ExpectedWriteMatchOutcome::Ambiguous(candidate_ids))
    }
}

fn fallback_expected_write_candidates(
    records: &[ExpectedWriteRecord],
    envelope: &GuardEnvelope,
    require_missing_host_invocation_id: bool,
) -> Vec<ExpectedWriteRecord> {
    let Some(session_id) = envelope.session_id.as_deref() else {
        return Vec::new();
    };
    records
        .iter()
        .filter(|record| record.session_id.as_deref() == Some(session_id))
        .filter(|record| !require_missing_host_invocation_id || record.host_invocation_id.is_none())
        .cloned()
        .collect()
}

fn expected_write_session_matches(record: &ExpectedWriteRecord, envelope: &GuardEnvelope) -> bool {
    envelope
        .session_id
        .as_deref()
        .is_none_or(|session_id| record.session_id.as_deref() == Some(session_id))
}

fn host_invocation_id_from_observation(observation: &ToolObservation) -> Option<String> {
    observation.host_invocation_id.clone()
}

fn expected_write_time_contains(record: &ExpectedWriteRecord, observed_at: DateTime<Utc>) -> bool {
    let Ok(created_at) = DateTime::parse_from_rfc3339(&record.created_at) else {
        return false;
    };
    let Ok(expires_at) = DateTime::parse_from_rfc3339(&record.expires_at) else {
        return false;
    };
    created_at.with_timezone(&Utc) <= observed_at && observed_at <= expires_at.with_timezone(&Utc)
}

fn expected_paths_cover_observed(
    record: &ExpectedWriteRecord,
    changed_set: &BTreeSet<String>,
) -> bool {
    if record.path_policy != "exact_paths" {
        return false;
    }
    let expected = json_string_set(&record.expected_paths_json);
    !changed_set.is_empty() && changed_set.is_subset(&expected)
}

fn matched_paths_cover_observed(
    record: &ExpectedWriteRecord,
    changed_set: &BTreeSet<String>,
) -> bool {
    let expected = record
        .matched_paths_json
        .as_deref()
        .map(json_string_set)
        .unwrap_or_default();
    !changed_set.is_empty() && changed_set.is_subset(&expected)
}

fn json_string_set(text: &str) -> BTreeSet<String> {
    serde_json::from_str::<Vec<String>>(text)
        .unwrap_or_default()
        .into_iter()
        .collect()
}

fn matched_expected_write_json(record: &ExpectedWriteRecord, changed: &[String]) -> Value {
    json!({
        "expected_write_id": record.expected_write_id,
        "status": "matched",
        "pre_tool_guard_event_id": record.pre_tool_guard_event_id,
        "host_invocation_id": record.host_invocation_id,
        "path_policy": record.path_policy,
        "observed_paths": changed,
        "task_id": record.task_id,
        "change_unit_id": record.change_unit_id,
        "ticket_backed": true,
        "write_ticket_ids": serde_json::from_str::<Value>(&record.write_ticket_ids_json)
            .unwrap_or_else(|_| json!([]))
    })
}

fn ticket_backed_observation_json(ticket: &ActiveWriteTicketSummary, changed: &[String]) -> Value {
    json!({
        "status": "ticket_backed",
        "ticket_backed": true,
        "write_ticket_id": ticket.write_ticket_id.clone(),
        "write_ticket_ids": [ticket.write_ticket_id.clone()],
        "observed_paths": changed,
        "change_unit_id": ticket.change_unit_id.clone(),
        "intended_paths": ticket.intended_paths.clone(),
        "expires_at": ticket.expires_at.clone()
    })
}
