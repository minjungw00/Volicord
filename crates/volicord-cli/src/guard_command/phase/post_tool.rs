use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path},
    process::{Command, Stdio},
};

use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_store::{
    bootstrap::ProjectRecord,
    core_pipeline::{CoreProjectStore, RunObservedChangesRecord},
    guards::{
        agent_session, insert_unrecorded_change, list_expected_writes_matched_by_post_event,
        list_pending_expected_writes, list_unresolved_unrecorded_changes,
        mark_expected_write_matched, post_tool_guard_events_for_session_since,
        promote_suspected_unrecorded_change, recorded_run_write_ticket_consumption,
        resolve_unrecorded_change, unrecorded_change, ExpectedWriteMatch, ExpectedWriteRecord,
        GuardEventRecord, UnrecordedChangeInsert, UnrecordedChangePromotion,
        UnrecordedChangeResolution, POST_TOOL_CORRELATION_EVENT_LIMIT,
    },
    StoreError,
};
use volicord_types::{
    canonical_git_object_id, canonical_json_bare_sha256, GuardDecision, ProjectId, TaskId,
    UnrecordedChangeConfidence, UnrecordedChangeResolutionBasis, WriteTicketValidityBasis,
    RECORD_RUN_TOOL_NAME,
};

use super::GuardPhaseResult;
use crate::guard_command::{
    context::{guard_state_summary, ActiveWriteTicketSummary, GuardStateSummary},
    envelope::{event_time, GuardEnvelope},
    json_error,
    mutation::{assess_reported_path, PathAssessment, ToolClassification},
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
    resolved_suspected_changes: Vec<Value>,
    recorded_change_suppressions: Vec<Value>,
    recorded_change_suppression_outcome: SuppressionOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SuppressionOutcome {
    Applied {
        remaining_paths: Vec<String>,
        suppressions: Vec<Value>,
    },
    Unavailable {
        remaining_paths: Vec<String>,
        reason: SuppressionUnavailableReason,
        scan_budget: usize,
        observed_count: usize,
    },
}

impl SuppressionOutcome {
    const fn status(&self) -> &'static str {
        match self {
            Self::Applied { .. } => "applied",
            Self::Unavailable { .. } => "unavailable",
        }
    }

    fn remaining_paths(&self) -> &[String] {
        match self {
            Self::Applied {
                remaining_paths, ..
            }
            | Self::Unavailable {
                remaining_paths, ..
            } => remaining_paths,
        }
    }

    fn suppressions(&self) -> &[Value] {
        match self {
            Self::Applied { suppressions, .. } => suppressions,
            Self::Unavailable { .. } => &[],
        }
    }

    const fn is_unavailable(&self) -> bool {
        matches!(self, Self::Unavailable { .. })
    }

    fn to_json(&self) -> Value {
        match self {
            Self::Applied {
                remaining_paths,
                suppressions,
            } => json!({
                "status": self.status(),
                "remaining_paths": remaining_paths,
                "suppressions": suppressions,
            }),
            Self::Unavailable {
                remaining_paths,
                reason,
                scan_budget,
                observed_count,
            } => json!({
                "status": self.status(),
                "remaining_paths": remaining_paths,
                "reason": reason.as_str(),
                "scan_budget": scan_budget,
                "observed_count": observed_count,
            }),
        }
    }

    fn diagnostic_json(&self) -> Option<Value> {
        let Self::Unavailable {
            reason,
            scan_budget,
            observed_count,
            ..
        } = self
        else {
            return None;
        };
        Some(json!({
            "code": reason.as_str(),
            "severity": "warning",
            "category": "recorded_change_suppression",
            "scan_budget": scan_budget,
            "observed_count": observed_count,
            "message": "Recorded-change suppression was unavailable; every observed path was retained for normal Guard correlation."
        }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SuppressionUnavailableReason {
    EventWindowExceeded,
    StoreReadFailed,
    StoredEventCorrupt,
    CorrelationPayloadInvalid,
    RunLookupFailed,
    WriteTicketLookupFailed,
    PathIdentityFailed,
}

impl SuppressionUnavailableReason {
    const fn as_str(self) -> &'static str {
        match self {
            Self::EventWindowExceeded => "event_window_exceeded",
            Self::StoreReadFailed => "store_read_failed",
            Self::StoredEventCorrupt => "stored_event_corrupt",
            Self::CorrelationPayloadInvalid => "correlation_payload_invalid",
            Self::RunLookupFailed => "run_lookup_failed",
            Self::WriteTicketLookupFailed => "write_ticket_lookup_failed",
            Self::PathIdentityFailed => "path_identity_failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SuppressionFailure {
    reason: SuppressionUnavailableReason,
    observed_count: usize,
}

impl SuppressionFailure {
    const fn new(reason: SuppressionUnavailableReason) -> Self {
        Self {
            reason,
            observed_count: 0,
        }
    }

    const fn with_observed_count(mut self, observed_count: usize) -> Self {
        self.observed_count = observed_count;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObservedChanges {
    paths: Vec<PathAssessment>,
    confidence: UnrecordedChangeConfidence,
    observation_confidence: &'static str,
    source: &'static str,
    confirms_no_change: bool,
}

struct UnrecordedChangeContext<'a> {
    runtime_home: &'a Path,
    project: &'a ProjectRecord,
    envelope: &'a GuardEnvelope,
    summary: &'a GuardStateSummary,
    observation: &'a ToolObservation,
    changed: Vec<String>,
    confidence: UnrecordedChangeConfidence,
    observation_confidence: &'static str,
    observation_source: &'static str,
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

const CORRELATED_PATH_IDENTITY_SCHEMA: &str = volicord_types::CORRELATED_PATH_IDENTITY_CONTRACT_ID;
const CORRELATED_PATH_IDENTITY_ALGORITHM: &str = "sha256";
const MAX_CORRELATED_FILE_BYTES: u64 = 16 * 1024 * 1024;

pub(in crate::guard_command) fn handle_post_tool(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    input: &crate::guard_command::args::GuardInput,
) -> Result<GuardPhaseResult, GuardCommandError> {
    let summary = guard_state_summary(runtime_home, project, envelope, input)?;
    let mut observation = tool_observation(&input.raw_value, &project.repo_root);
    let observed_changes = observed_changes(runtime_home, project, envelope, &observation)?;
    observation.changed_paths = observed_changes.paths.clone();
    observation.changed_paths_reported =
        observed_changes.confidence == UnrecordedChangeConfidence::Confirmed;
    let correlation = record_post_tool_correlation(
        runtime_home,
        project,
        envelope,
        &summary,
        &observation,
        &observed_changes,
    )?;
    let suppression_unavailable = correlation
        .recorded_change_suppression_outcome
        .is_unavailable();
    let suppression_outcome_json = correlation.recorded_change_suppression_outcome.to_json();
    let suppression_diagnostics = correlation
        .recorded_change_suppression_outcome
        .diagnostic_json()
        .into_iter()
        .collect::<Vec<_>>();
    let suppression_reasons = suppression_diagnostics.clone();
    let decision = if correlation.unrecorded_changes.is_empty() && !suppression_unavailable {
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
            "resolved_suspected_changes": correlation.resolved_suspected_changes,
            "recorded_change_suppressions": correlation.recorded_change_suppressions,
            "recorded_change_suppression_outcome": suppression_outcome_json,
            "diagnostics": suppression_diagnostics,
            "reasons": suppression_reasons,
            "change_observation": {
                "source": observed_changes.source,
                "confidence": observed_changes.observation_confidence,
                "confirms_no_change": observed_changes.confirms_no_change
            },
            "context": context_json(&summary),
            "enforcement_level": "cooperative_guard"
        }),
    ))
}

fn record_post_tool_correlation(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    summary: &GuardStateSummary,
    observation: &ToolObservation,
    observed_changes: &ObservedChanges,
) -> Result<PostToolCorrelation, GuardCommandError> {
    if observation.tool_name.as_deref() == Some(RECORD_RUN_TOOL_NAME) {
        return Ok(PostToolCorrelation {
            matched_expected_writes: Vec::new(),
            ticket_backed_observations: Vec::new(),
            unrecorded_changes: Vec::new(),
            resolved_suspected_changes: Vec::new(),
            recorded_change_suppressions: Vec::new(),
            recorded_change_suppression_outcome: SuppressionOutcome::Applied {
                remaining_paths: Vec::new(),
                suppressions: Vec::new(),
            },
        });
    }
    let observed_paths = normalized_observed_paths(observation.changed_paths.iter());
    let recorded_change_suppression_outcome = if observed_paths.is_empty() {
        SuppressionOutcome::Applied {
            remaining_paths: Vec::new(),
            suppressions: Vec::new(),
        }
    } else if observed_changes.confidence == UnrecordedChangeConfidence::Confirmed
        && matches!(
            observed_changes.source,
            "structured_host_changed_paths" | "git_worktree_diff"
        )
    {
        suppress_previously_recorded_changes(
            runtime_home,
            project,
            envelope,
            observed_changes.source,
            &observed_paths,
        )
    } else {
        SuppressionOutcome::Unavailable {
            remaining_paths: observed_paths,
            reason: SuppressionUnavailableReason::CorrelationPayloadInvalid,
            scan_budget: POST_TOOL_CORRELATION_EVENT_LIMIT,
            observed_count: 0,
        }
    };
    let changed = recorded_change_suppression_outcome
        .remaining_paths()
        .to_vec();
    let recorded_change_suppressions = recorded_change_suppression_outcome.suppressions().to_vec();
    if changed.is_empty() {
        let confirms_no_new_change =
            observed_changes.confirms_no_change || !recorded_change_suppressions.is_empty();
        let resolved_suspected_changes = if confirms_no_new_change {
            resolve_matching_suspected_no_change(
                runtime_home,
                project,
                envelope,
                summary,
                observation,
                observed_changes.source,
            )?
        } else {
            Vec::new()
        };
        let unrecorded_changes = if !confirms_no_new_change && possible_product_write(observation) {
            record_unrecorded_changes(UnrecordedChangeContext {
                runtime_home,
                project,
                envelope,
                summary,
                observation,
                changed,
                confidence: UnrecordedChangeConfidence::Suspected,
                observation_confidence: observed_changes.observation_confidence,
                observation_source: observed_changes.source,
                correlation_status: "effect_unconfirmed",
                candidate_expected_write_ids: Vec::new(),
            })?
        } else {
            Vec::new()
        };
        return Ok(PostToolCorrelation {
            matched_expected_writes: Vec::new(),
            ticket_backed_observations: Vec::new(),
            unrecorded_changes,
            resolved_suspected_changes,
            recorded_change_suppressions,
            recorded_change_suppression_outcome,
        });
    }
    if observed_changes.confidence == UnrecordedChangeConfidence::Suspected {
        return Ok(PostToolCorrelation {
            matched_expected_writes: Vec::new(),
            ticket_backed_observations: Vec::new(),
            unrecorded_changes: record_unrecorded_changes(UnrecordedChangeContext {
                runtime_home,
                project,
                envelope,
                summary,
                observation,
                changed,
                confidence: UnrecordedChangeConfidence::Suspected,
                observation_confidence: observed_changes.observation_confidence,
                observation_source: observed_changes.source,
                correlation_status: "heuristic_paths_only",
                candidate_expected_write_ids: Vec::new(),
            })?,
            resolved_suspected_changes: Vec::new(),
            recorded_change_suppressions,
            recorded_change_suppression_outcome,
        });
    }
    let match_outcome =
        match_expected_write(runtime_home, project, envelope, observation, &changed)?;
    match match_outcome {
        ExpectedWriteMatchOutcome::Matched(record) => {
            let repository_identity = correlated_path_identity(
                runtime_home,
                project,
                envelope,
                observed_changes.source,
                &changed,
            )
            .unwrap_or(None);
            mark_expected_write_matched(
                runtime_home,
                &project.project_id,
                &record.expected_write_id,
                ExpectedWriteMatch {
                    matched_post_tool_guard_event_id: envelope.event_id.clone(),
                    matched_paths: changed.clone(),
                    matched_at: envelope.occurred_at.clone(),
                },
            )?;
            let resolved_suspected_changes = resolve_matching_suspected_authorized(
                runtime_home,
                project,
                envelope,
                summary,
                observation,
                observed_changes.source,
                UnrecordedChangeResolutionBasis::RecordedAsExpectedWrite,
                &record.expected_write_id,
            )?;
            Ok(PostToolCorrelation {
                matched_expected_writes: vec![matched_expected_write_json(
                    &record,
                    &changed,
                    repository_identity.as_ref(),
                )],
                ticket_backed_observations: Vec::new(),
                unrecorded_changes: Vec::new(),
                resolved_suspected_changes,
                recorded_change_suppressions,
                recorded_change_suppression_outcome,
            })
        }
        ExpectedWriteMatchOutcome::AlreadyMatched(record) => {
            let repository_identity = correlated_path_identity(
                runtime_home,
                project,
                envelope,
                observed_changes.source,
                &changed,
            )
            .unwrap_or(None);
            let resolved_suspected_changes = resolve_matching_suspected_authorized(
                runtime_home,
                project,
                envelope,
                summary,
                observation,
                observed_changes.source,
                UnrecordedChangeResolutionBasis::RecordedAsExpectedWrite,
                &record.expected_write_id,
            )?;
            Ok(PostToolCorrelation {
                matched_expected_writes: vec![matched_expected_write_json(
                    &record,
                    &changed,
                    repository_identity.as_ref(),
                )],
                ticket_backed_observations: Vec::new(),
                unrecorded_changes: Vec::new(),
                resolved_suspected_changes,
                recorded_change_suppressions,
                recorded_change_suppression_outcome,
            })
        }
        ExpectedWriteMatchOutcome::NoCandidates => {
            match active_write_ticket_match(summary, &changed) {
                ActiveWriteTicketMatchOutcome::Matched(ticket) => {
                    let repository_identity = correlated_path_identity(
                        runtime_home,
                        project,
                        envelope,
                        observed_changes.source,
                        &changed,
                    )
                    .unwrap_or(None);
                    let resolved_suspected_changes = resolve_matching_suspected_authorized(
                        runtime_home,
                        project,
                        envelope,
                        summary,
                        observation,
                        observed_changes.source,
                        UnrecordedChangeResolutionBasis::CoveredByWriteTicket,
                        &ticket.write_ticket_id,
                    )?;
                    Ok(PostToolCorrelation {
                        matched_expected_writes: Vec::new(),
                        ticket_backed_observations: vec![ticket_backed_observation_json(
                            &ticket,
                            &changed,
                            repository_identity.as_ref(),
                        )],
                        unrecorded_changes: Vec::new(),
                        resolved_suspected_changes,
                        recorded_change_suppressions,
                        recorded_change_suppression_outcome,
                    })
                }
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
                        confidence: UnrecordedChangeConfidence::Confirmed,
                        observation_confidence: observed_changes.observation_confidence,
                        observation_source: observed_changes.source,
                        correlation_status: "unmatched_expected_write",
                        candidate_expected_write_ids: Vec::new(),
                    })?,
                    resolved_suspected_changes: Vec::new(),
                    recorded_change_suppressions,
                    recorded_change_suppression_outcome,
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
                        confidence: UnrecordedChangeConfidence::Confirmed,
                        observation_confidence: observed_changes.observation_confidence,
                        observation_source: observed_changes.source,
                        correlation_status: "out_of_scope_write_ticket",
                        candidate_expected_write_ids: ticket_ids,
                    })?,
                    resolved_suspected_changes: Vec::new(),
                    recorded_change_suppressions,
                    recorded_change_suppression_outcome,
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
                        confidence: UnrecordedChangeConfidence::Confirmed,
                        observation_confidence: observed_changes.observation_confidence,
                        observation_source: observed_changes.source,
                        correlation_status: "ambiguous_write_ticket",
                        candidate_expected_write_ids: ticket_ids,
                    })?,
                    resolved_suspected_changes: Vec::new(),
                    recorded_change_suppressions,
                    recorded_change_suppression_outcome,
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
                confidence: UnrecordedChangeConfidence::Confirmed,
                observation_confidence: observed_changes.observation_confidence,
                observation_source: observed_changes.source,
                correlation_status: "out_of_scope_expected_write",
                candidate_expected_write_ids: candidate_ids,
            })?,
            resolved_suspected_changes: Vec::new(),
            recorded_change_suppressions,
            recorded_change_suppression_outcome,
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
                confidence: UnrecordedChangeConfidence::Confirmed,
                observation_confidence: observed_changes.observation_confidence,
                observation_source: observed_changes.source,
                correlation_status: "ambiguous_expected_write",
                candidate_expected_write_ids: candidate_ids,
            })?,
            resolved_suspected_changes: Vec::new(),
            recorded_change_suppressions,
            recorded_change_suppression_outcome,
        }),
    }
}

fn record_unrecorded_changes(
    context: UnrecordedChangeContext<'_>,
) -> Result<Vec<Value>, GuardCommandError> {
    if context.changed.is_empty() && context.confidence == UnrecordedChangeConfidence::Confirmed {
        return Ok(Vec::new());
    }
    if context.confidence == UnrecordedChangeConfidence::Confirmed {
        let promoted = promote_matching_suspected_changes(&context)?;
        if !promoted.is_empty() {
            return Ok(promoted);
        }
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
            "confidence": context.confidence.as_str(),
            "observed_paths": context.changed,
            "correlation_status": context.correlation_status
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
            confidence: context.confidence.as_str().to_owned(),
            summary: "Product file changes were observed after a host tool without a matching Volicord run record".to_owned(),
            observed_paths_json: serde_json::to_string(&context.changed).map_err(json_error)?,
            detection_json: json!({
                "source": "volicord_guard_post_tool",
                "observation_source": context.observation_source,
                "observation_confidence": context.observation_confidence,
                "unrecorded_change_confidence": context.confidence.as_str(),
                "host_invocation_id": context.observation.host_invocation_id,
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
                "observer_role": "guard",
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
        "confidence": context.confidence.as_str(),
        "observed_paths": context.changed,
        "correlation_status": context.correlation_status
    })])
}

fn promote_matching_suspected_changes(
    context: &UnrecordedChangeContext<'_>,
) -> Result<Vec<Value>, GuardCommandError> {
    let Some(host_invocation_id) = context.observation.host_invocation_id.as_deref() else {
        return Ok(Vec::new());
    };
    let observed_paths_json = serde_json::to_string(&context.changed).map_err(json_error)?;
    let mut promoted = Vec::new();
    for record in list_unresolved_unrecorded_changes(
        context.runtime_home,
        &context.project.project_id,
        Some(&context.envelope.connection_id),
    )? {
        if record.confidence != UnrecordedChangeConfidence::Suspected.as_str()
            || record.session_id != context.envelope.session_id
            || record.task_id != context.summary.active_task_id
        {
            continue;
        }
        let mut detection =
            serde_json::from_str::<Value>(&record.detection_json).map_err(json_error)?;
        if detection.get("host_invocation_id").and_then(Value::as_str) != Some(host_invocation_id) {
            continue;
        }
        let Some(detection_object) = detection.as_object_mut() else {
            return Err(GuardCommandError::Runtime(
                "stored unrecorded-change detection must be an object".to_owned(),
            ));
        };
        let prior_observation_confidence = detection_object
            .get("observation_confidence")
            .cloned()
            .unwrap_or(Value::Null);
        detection_object.insert(
            "observation_source".to_owned(),
            Value::String(context.observation_source.to_owned()),
        );
        detection_object.insert(
            "observation_confidence".to_owned(),
            Value::String(context.observation_confidence.to_owned()),
        );
        detection_object.insert(
            "unrecorded_change_confidence".to_owned(),
            Value::String(UnrecordedChangeConfidence::Confirmed.as_str().to_owned()),
        );
        detection_object.insert(
            "correlation_status".to_owned(),
            Value::String(context.correlation_status.to_owned()),
        );
        detection_object.insert(
            "candidate_expected_write_ids".to_owned(),
            serde_json::to_value(&context.candidate_expected_write_ids).map_err(json_error)?,
        );
        detection_object.insert(
            "promotion".to_owned(),
            json!({
                "basis": "deterministic_post_tool_observation",
                "source": context.observation_source,
                "confirmed_at": context.envelope.occurred_at,
                "prior_observation_confidence": prior_observation_confidence
            }),
        );
        promote_suspected_unrecorded_change(
            context.runtime_home,
            &context.project.project_id,
            &record.unrecorded_change_id,
            UnrecordedChangePromotion {
                observed_paths_json: observed_paths_json.clone(),
                detection_json: detection.to_string(),
                confirmed_at: context.envelope.occurred_at.clone(),
            },
        )?;
        promoted.push(json!({
            "unrecorded_change_id": record.unrecorded_change_id,
            "status": "promoted",
            "confidence": "confirmed",
            "observed_paths": context.changed,
            "correlation_status": context.correlation_status
        }));
    }
    Ok(promoted)
}

fn possible_product_write(observation: &ToolObservation) -> bool {
    observation.explicit_write_attempt
        || matches!(
            observation.classification,
            ToolClassification::Mutating | ToolClassification::UnknownMutationRisk
        )
        || observation.effect() == "product_file_write"
}

fn correlated_path_identity(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    _observation_source: &str,
    paths: &[String],
) -> Result<Option<Value>, GuardCommandError> {
    if paths.is_empty() {
        return Ok(None);
    }
    let Some(baseline_identity) = correlation_baseline_identity(runtime_home, project, envelope)?
    else {
        return Ok(None);
    };
    let Some(snapshot_digest) = exact_paths_digest(&project.repo_root, paths) else {
        return Ok(None);
    };
    Ok(Some(json!({
        "schema": CORRELATED_PATH_IDENTITY_SCHEMA,
        "algorithm": CORRELATED_PATH_IDENTITY_ALGORITHM,
        "scope": "exact_observed_paths",
        "observed_paths": paths,
        "snapshot_digest": snapshot_digest,
        "baseline_identity": baseline_identity
    })))
}

fn correlation_baseline_identity(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
) -> Result<Option<Value>, GuardCommandError> {
    let Some(session_id) = envelope.session_id.as_deref() else {
        return Ok(None);
    };
    let Some(session) = agent_session(runtime_home, &project.project_id, session_id)?
        .filter(|session| session.connection_internal_id == envelope.connection_id)
    else {
        return Ok(None);
    };
    let Some(head_oid) = git_head_oid(&project.repo_root) else {
        return Ok(None);
    };
    Ok(Some(json!({
        "kind": "git_worktree",
        "session_id": session.session_id,
        "connection_internal_id": session.connection_internal_id,
        "session_first_observed_at": session.first_observed_at,
        "head_oid": head_oid
    })))
}

fn exact_paths_digest(repo_root: &Path, paths: &[String]) -> Option<String> {
    let mut entries = Vec::with_capacity(paths.len());
    for path in paths {
        let relative = Path::new(path);
        if relative.is_absolute()
            || relative.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return None;
        }
        let absolute = repo_root.join(relative);
        let metadata = match fs::symlink_metadata(&absolute) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                entries.push(json!({"path": path, "kind": "missing"}));
                continue;
            }
            Err(_) => return None,
        };
        if metadata.file_type().is_symlink() {
            let target = fs::read_link(&absolute).ok()?;
            entries.push(json!({
                "path": path,
                "kind": "symlink",
                "target": target.to_string_lossy()
            }));
        } else if metadata.is_file() {
            if metadata.len() > MAX_CORRELATED_FILE_BYTES {
                return None;
            }
            let bytes = fs::read(&absolute).ok()?;
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            entries.push(json!({
                "path": path,
                "kind": "file",
                "size": metadata.len(),
                "sha256": format!("{:x}", hasher.finalize())
            }));
        } else if metadata.is_dir() {
            entries.push(json!({"path": path, "kind": "directory"}));
        } else {
            return None;
        }
    }
    Some(
        canonical_json_bare_sha256(&entries)
            .expect("filesystem identity entries always have a canonical JSON encoding"),
    )
}
fn git_head_oid(repo_root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("--no-optional-locks")
        .arg("-C")
        .arg(repo_root)
        .args(["rev-parse", "--verify", "HEAD"])
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() || output.stdout.len() > 128 {
        return None;
    }
    let value = std::str::from_utf8(&output.stdout).ok()?.trim();
    canonical_git_object_id(value).ok()
}

fn suppress_previously_recorded_changes(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    observation_source: &str,
    observed_paths: &[String],
) -> SuppressionOutcome {
    match try_suppress_previously_recorded_changes(
        runtime_home,
        project,
        envelope,
        observation_source,
        observed_paths,
    ) {
        Ok(outcome) => outcome,
        Err(failure) => SuppressionOutcome::Unavailable {
            remaining_paths: observed_paths.to_vec(),
            reason: failure.reason,
            scan_budget: POST_TOOL_CORRELATION_EVENT_LIMIT,
            observed_count: failure.observed_count,
        },
    }
}

fn try_suppress_previously_recorded_changes(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    observation_source: &str,
    observed_paths: &[String],
) -> Result<SuppressionOutcome, SuppressionFailure> {
    let session_id = envelope.session_id.as_deref().ok_or_else(|| {
        SuppressionFailure::new(SuppressionUnavailableReason::CorrelationPayloadInvalid)
    })?;
    let baseline_identity = correlation_baseline_identity(runtime_home, project, envelope)
        .map_err(|_| SuppressionFailure::new(SuppressionUnavailableReason::StoreReadFailed))?
        .ok_or_else(|| SuppressionFailure::new(SuppressionUnavailableReason::PathIdentityFailed))?;
    let not_before = baseline_identity
        .get("created_at")
        .or_else(|| baseline_identity.get("session_first_observed_at"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            SuppressionFailure::new(SuppressionUnavailableReason::CorrelationPayloadInvalid)
        })?;
    let current_at = parsed_timestamp(&envelope.occurred_at).ok_or_else(|| {
        SuppressionFailure::new(SuppressionUnavailableReason::CorrelationPayloadInvalid)
    })?;
    let events = post_tool_guard_events_for_session_since(
        runtime_home,
        &project.project_id,
        session_id,
        &envelope.connection_id,
        not_before,
    )
    .map_err(correlation_event_query_failure)?;
    let observed_count = events.len();
    let store = CoreProjectStore::open(runtime_home, &ProjectId::new(&project.project_id))
        .map_err(|_| {
            SuppressionFailure::new(SuppressionUnavailableReason::StoreReadFailed)
                .with_observed_count(observed_count)
        })?;
    let observed_set = observed_paths.iter().cloned().collect::<BTreeSet<_>>();
    let mut suppressed = BTreeSet::new();
    let mut suppressions = Vec::new();
    let mut current_identities = BTreeMap::<Vec<String>, Option<Value>>::new();
    let mut run_observations = BTreeMap::<String, Vec<RunObservedChangesRecord>>::new();
    for event in events {
        if event.guard_event_id == envelope.event_id {
            continue;
        }
        let event_at = parsed_timestamp(&event.occurred_at).ok_or_else(|| {
            SuppressionFailure::new(SuppressionUnavailableReason::StoredEventCorrupt)
                .with_observed_count(observed_count)
        })?;
        if event_at > current_at {
            continue;
        }
        match event.decision.as_str() {
            "allow" => {}
            "warn" | "deny" | "inject_context" => continue,
            _ => {
                return Err(SuppressionFailure::new(
                    SuppressionUnavailableReason::StoredEventCorrupt,
                )
                .with_observed_count(observed_count))
            }
        }
        if !guard_event_source_binding_is_valid(&event) {
            return Err(
                SuppressionFailure::new(SuppressionUnavailableReason::StoredEventCorrupt)
                    .with_observed_count(observed_count),
            );
        }
        let result = serde_json::from_str::<Value>(&event.result_json).map_err(|_| {
            SuppressionFailure::new(SuppressionUnavailableReason::StoredEventCorrupt)
                .with_observed_count(observed_count)
        })?;
        if !result.is_object() {
            return Err(
                SuppressionFailure::new(SuppressionUnavailableReason::StoredEventCorrupt)
                    .with_observed_count(observed_count),
            );
        }
        let entries = durable_correlation_entries(&result).map_err(|_| {
            SuppressionFailure::new(SuppressionUnavailableReason::CorrelationPayloadInvalid)
                .with_observed_count(observed_count)
        })?;
        if entries.is_empty() {
            continue;
        }
        for entry in entries {
            let (paths, ticket_ids, expected_identity) = durable_correlation_checkpoint(entry)
                .ok_or_else(|| {
                    SuppressionFailure::new(SuppressionUnavailableReason::CorrelationPayloadInvalid)
                        .with_observed_count(observed_count)
                })?;
            let path_set = paths.iter().cloned().collect::<BTreeSet<_>>();
            if !path_set.is_subset(&observed_set) {
                continue;
            }
            let Some((write_ticket_id, run_id)) = committed_run_for_correlation(
                runtime_home,
                project,
                &store,
                &ticket_ids,
                &path_set,
                event_at,
                &mut run_observations,
            )
            .map_err(|failure| failure.with_observed_count(observed_count))?
            else {
                continue;
            };
            let current_identity = if let Some(identity) = current_identities.get(&paths) {
                identity.clone()
            } else {
                let identity = correlated_path_identity(
                    runtime_home,
                    project,
                    envelope,
                    observation_source,
                    &paths,
                )
                .map_err(|_| {
                    SuppressionFailure::new(SuppressionUnavailableReason::PathIdentityFailed)
                        .with_observed_count(observed_count)
                })?;
                current_identities.insert(paths.clone(), identity.clone());
                identity
            };
            if current_identity.is_none() {
                return Err(SuppressionFailure::new(
                    SuppressionUnavailableReason::PathIdentityFailed,
                )
                .with_observed_count(observed_count));
            }
            if current_identity.as_ref() != Some(expected_identity) {
                continue;
            }
            suppressed.extend(paths.iter().cloned());
            suppressions.push(json!({
                "status": "recorded_identity_unchanged",
                "guard_event_id": event.guard_event_id,
                "write_ticket_id": write_ticket_id,
                "run_id": run_id,
                "observed_paths": paths,
                "identity_schema": CORRELATED_PATH_IDENTITY_SCHEMA
            }));
        }
    }
    Ok(SuppressionOutcome::Applied {
        remaining_paths: observed_set.difference(&suppressed).cloned().collect(),
        suppressions,
    })
}

fn correlation_event_query_failure(error: StoreError) -> SuppressionFailure {
    match error {
        StoreError::InvalidInput { detail }
            if detail.contains("post-tool correlation window exceeds") =>
        {
            SuppressionFailure::new(SuppressionUnavailableReason::EventWindowExceeded)
                .with_observed_count(POST_TOOL_CORRELATION_EVENT_LIMIT + 1)
        }
        _ => SuppressionFailure::new(SuppressionUnavailableReason::StoreReadFailed),
    }
}

fn durable_correlation_entries(result: &Value) -> Result<Vec<&Value>, ()> {
    let mut entries = Vec::new();
    for field in ["matched_expected_writes", "ticket_backed_observations"] {
        match result.get(field) {
            Some(Value::Array(values)) => entries.extend(values),
            Some(_) => return Err(()),
            None => {}
        }
    }
    Ok(entries)
}

fn guard_event_source_binding_is_valid(event: &GuardEventRecord) -> bool {
    let Ok(metadata) = serde_json::from_str::<Value>(&event.metadata_json) else {
        return false;
    };
    let Some(expected) = metadata
        .get("source_payload_sha256")
        .and_then(Value::as_str)
        .filter(|value| valid_sha256_coordinate(value))
    else {
        return false;
    };
    let Ok(subject) = serde_json::from_str::<Value>(&event.subject_json) else {
        return false;
    };
    let Some(raw_event_sha256) = subject
        .get("raw_event_sha256")
        .and_then(Value::as_str)
        .filter(|value| valid_sha256_coordinate(value))
    else {
        return false;
    };
    canonical_json_bare_sha256(&json!({
        "session_id": event.session_id,
        "connection_id": event.connection_internal_id,
        "guard_installation_id": event.guard_installation_id,
        "event_kind": event.event_kind,
        "raw_event_sha256": raw_event_sha256
    }))
    .is_ok_and(|actual| actual == expected)
}

fn durable_correlation_checkpoint(entry: &Value) -> Option<(Vec<String>, Vec<String>, &Value)> {
    if entry.get("ticket_backed").and_then(Value::as_bool) != Some(true)
        || !matches!(
            entry.get("status").and_then(Value::as_str),
            Some("matched" | "ticket_backed")
        )
    {
        return None;
    }
    let paths = strict_nonempty_string_set(entry.get("observed_paths")?)?;
    let ticket_ids = strict_nonempty_string_set(entry.get("write_ticket_ids")?)?;
    let identity = entry.get("repository_identity")?;
    let identity_object = identity.as_object()?;
    if identity_object.get("schema").and_then(Value::as_str)
        != Some(CORRELATED_PATH_IDENTITY_SCHEMA)
        || identity_object.get("algorithm").and_then(Value::as_str)
            != Some(CORRELATED_PATH_IDENTITY_ALGORITHM)
        || identity_object.get("scope").and_then(Value::as_str) != Some("exact_observed_paths")
        || strict_nonempty_string_set(identity_object.get("observed_paths")?)? != paths
        || !identity_object
            .get("baseline_identity")
            .is_some_and(Value::is_object)
        || !identity_object
            .get("snapshot_digest")
            .and_then(Value::as_str)
            .is_some_and(valid_lowercase_sha256)
    {
        return None;
    }
    Some((paths, ticket_ids, identity))
}

fn committed_run_for_correlation(
    runtime_home: &Path,
    project: &ProjectRecord,
    store: &CoreProjectStore,
    ticket_ids: &[String],
    paths: &BTreeSet<String>,
    correlated_at: DateTime<Utc>,
    run_observations: &mut BTreeMap<String, Vec<RunObservedChangesRecord>>,
) -> Result<Option<(String, String)>, SuppressionFailure> {
    for ticket_id in ticket_ids {
        let Some(ticket) = store.write_ticket_record(ticket_id).map_err(|_| {
            SuppressionFailure::new(SuppressionUnavailableReason::WriteTicketLookupFailed)
        })?
        else {
            continue;
        };
        let (Some(run_id), Some(consumed_at)) = (
            ticket.consumed_by_run_id.as_deref(),
            ticket.consumed_at.as_deref().and_then(parsed_timestamp),
        ) else {
            continue;
        };
        let Some(created_at) = parsed_timestamp(&ticket.created_at) else {
            continue;
        };
        if ticket.status != "consumed" || created_at > correlated_at || consumed_at < correlated_at
        {
            continue;
        }
        let Some(run) = store
            .run_record(run_id)
            .map_err(|_| SuppressionFailure::new(SuppressionUnavailableReason::RunLookupFailed))?
        else {
            continue;
        };
        let Some(run_ticket_effect) =
            recorded_run_write_ticket_consumption(runtime_home, &project.project_id, run_id)
                .map_err(|_| {
                    SuppressionFailure::new(SuppressionUnavailableReason::RunLookupFailed)
                })?
        else {
            continue;
        };
        let validity_basis: WriteTicketValidityBasis =
            serde_json::from_str(&ticket.validity_basis_json).map_err(|_| {
                SuppressionFailure::new(SuppressionUnavailableReason::WriteTicketLookupFailed)
            })?;
        if run.status != "recorded"
            || run.task_id != ticket.task_id
            || run.change_unit_id.as_deref() != Some(ticket.change_unit_id.as_str())
            || run_ticket_effect.write_ticket_id != ticket.write_ticket_id
            || validity_basis.task_id.as_str() != ticket.task_id
            || validity_basis.change_unit_id.as_str() != ticket.change_unit_id
            || validity_basis
                .baseline_ref
                .as_ref()
                .map(|value| value.as_str())
                != run.baseline_ref.as_deref()
            || validity_basis.scope_revision != run.scope_revision
        {
            continue;
        }
        if !run_observations.contains_key(&ticket.task_id) {
            run_observations.insert(
                ticket.task_id.clone(),
                store
                    .run_observed_changes_for_task(&TaskId::new(&ticket.task_id))
                    .map_err(|_| {
                        SuppressionFailure::new(SuppressionUnavailableReason::RunLookupFailed)
                    })?,
            );
        }
        let observations = run_observations
            .get(&ticket.task_id)
            .expect("task observations were inserted above");
        let Some(observed) = observations
            .iter()
            .find(|observed| observed.run_id == run_id && observed.status == "recorded")
        else {
            continue;
        };
        let run_paths = observed
            .observed_changes
            .changed_paths
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        if observed.observed_changes.product_file_write_observed && paths.is_subset(&run_paths) {
            return Ok(Some((ticket_id.clone(), run_id.to_owned())));
        }
    }
    Ok(None)
}

fn strict_nonempty_string_set(value: &Value) -> Option<Vec<String>> {
    let values = value.as_array()?;
    if values.is_empty() {
        return None;
    }
    let set = values
        .iter()
        .map(Value::as_str)
        .collect::<Option<BTreeSet<_>>>()?;
    if set.len() != values.len() || set.iter().any(|value| value.trim().is_empty()) {
        return None;
    }
    Some(set.into_iter().map(str::to_owned).collect())
}

fn valid_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_sha256_coordinate(value: &str) -> bool {
    valid_lowercase_sha256(value)
        || value
            .strip_prefix("sha256:")
            .is_some_and(valid_lowercase_sha256)
}

fn parsed_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn observed_changes(
    _runtime_home: &Path,
    project: &ProjectRecord,
    _envelope: &GuardEnvelope,
    observation: &ToolObservation,
) -> Result<ObservedChanges, GuardCommandError> {
    if observation.changed_paths_reported {
        return Ok(ObservedChanges {
            paths: observation.changed_paths.clone(),
            confidence: UnrecordedChangeConfidence::Confirmed,
            observation_confidence: "confirmed",
            source: "structured_host_changed_paths",
            confirms_no_change: observation.changed_paths.is_empty(),
        });
    }
    if let Some(paths) = git_worktree_changed_paths(&project.repo_root) {
        let confirms_no_change = paths.is_empty();
        return Ok(ObservedChanges {
            paths,
            confidence: UnrecordedChangeConfidence::Confirmed,
            observation_confidence: "confirmed",
            source: "git_worktree_diff",
            confirms_no_change,
        });
    }
    Ok(ObservedChanges {
        paths: observation.paths.clone(),
        confidence: UnrecordedChangeConfidence::Suspected,
        observation_confidence: if possible_product_write(observation) {
            "heuristic"
        } else {
            "unknown"
        },
        source: "heuristic_event",
        confirms_no_change: false,
    })
}

fn git_worktree_changed_paths(repo_root: &Path) -> Option<Vec<PathAssessment>> {
    const MAX_GIT_STATUS_BYTES: usize = 1024 * 1024;
    let output = Command::new("git")
        .arg("--no-optional-locks")
        .arg("-C")
        .arg(repo_root)
        .args([
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--",
            ".",
        ])
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() || output.stdout.len() > MAX_GIT_STATUS_BYTES {
        return None;
    }
    let mut paths = BTreeSet::new();
    let mut fields = output.stdout.split(|byte| *byte == 0).peekable();
    while let Some(field) = fields.next() {
        if field.is_empty() || field.len() < 4 || field[2] != b' ' {
            continue;
        }
        let status = &field[..2];
        if let Ok(path) = std::str::from_utf8(&field[3..]) {
            if !path.is_empty() {
                paths.insert(path.to_owned());
            }
        }
        if status.iter().any(|byte| matches!(*byte, b'R' | b'C')) {
            if let Some(other) = fields
                .next()
                .and_then(|field| std::str::from_utf8(field).ok())
            {
                if !other.is_empty() {
                    paths.insert(other.to_owned());
                }
            }
        }
    }
    const EXCLUSIONS: &[&str] = &[
        ".git",
        ".hg",
        ".svn",
        ".jj",
        ".volicord",
        "target",
        "node_modules",
        "dist",
        "build",
        "coverage",
        "vendor",
    ];
    Some(
        paths
            .into_iter()
            .filter(|path| {
                !EXCLUSIONS.iter().any(|excluded| {
                    path == *excluded
                        || path
                            .strip_prefix(excluded)
                            .is_some_and(|suffix| suffix.starts_with('/'))
                })
            })
            .map(|path| assess_reported_path(repo_root, &path))
            .collect(),
    )
}

fn resolve_matching_suspected_no_change(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    summary: &GuardStateSummary,
    observation: &ToolObservation,
    source: &str,
) -> Result<Vec<Value>, GuardCommandError> {
    let Some(host_invocation_id) = observation.host_invocation_id.as_deref() else {
        return Ok(Vec::new());
    };
    let resolved_at = envelope.occurred_at.clone();
    let mut resolved = Vec::new();
    for record in list_unresolved_unrecorded_changes(
        runtime_home,
        &project.project_id,
        Some(&envelope.connection_id),
    )? {
        if record.confidence != UnrecordedChangeConfidence::Suspected.as_str()
            || record.session_id != envelope.session_id
            || record.task_id != summary.active_task_id
        {
            continue;
        }
        let detection =
            serde_json::from_str::<Value>(&record.detection_json).map_err(json_error)?;
        if detection.get("host_invocation_id").and_then(Value::as_str) != Some(host_invocation_id) {
            continue;
        }
        resolve_unrecorded_change(
            runtime_home,
            &project.project_id,
            &record.unrecorded_change_id,
            UnrecordedChangeResolution {
                resolution_json: json!({
                    "resolution_basis": UnrecordedChangeResolutionBasis::InvalidObservation.as_str(),
                    "source": source,
                    "confirmed_no_product_change": true
                })
                .to_string(),
                resolved_at: resolved_at.clone(),
                resolved_by_actor_source: "system".to_owned(),
            },
        )?;
        resolved.push(json!({
            "unrecorded_change_id": record.unrecorded_change_id,
            "status": "resolved",
            "confidence": "suspected",
            "resolution_basis": "invalid_observation"
        }));
    }
    Ok(resolved)
}

#[allow(clippy::too_many_arguments)]
fn resolve_matching_suspected_authorized(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    summary: &GuardStateSummary,
    observation: &ToolObservation,
    source: &str,
    basis: UnrecordedChangeResolutionBasis,
    authority_id: &str,
) -> Result<Vec<Value>, GuardCommandError> {
    let Some(host_invocation_id) = observation.host_invocation_id.as_deref() else {
        return Ok(Vec::new());
    };
    let resolved_at = envelope.occurred_at.clone();
    let mut resolved = Vec::new();
    for record in list_unresolved_unrecorded_changes(
        runtime_home,
        &project.project_id,
        Some(&envelope.connection_id),
    )? {
        if record.confidence != UnrecordedChangeConfidence::Suspected.as_str()
            || record.session_id != envelope.session_id
            || record.task_id != summary.active_task_id
        {
            continue;
        }
        let detection =
            serde_json::from_str::<Value>(&record.detection_json).map_err(json_error)?;
        if detection.get("host_invocation_id").and_then(Value::as_str) != Some(host_invocation_id) {
            continue;
        }
        resolve_unrecorded_change(
            runtime_home,
            &project.project_id,
            &record.unrecorded_change_id,
            UnrecordedChangeResolution {
                resolution_json: json!({
                    "resolution_basis": basis.as_str(),
                    "source": source,
                    "authority_id": authority_id,
                    "deterministic_correlation": true
                })
                .to_string(),
                resolved_at: resolved_at.clone(),
                resolved_by_actor_source: "system".to_owned(),
            },
        )?;
        resolved.push(json!({
            "unrecorded_change_id": record.unrecorded_change_id,
            "status": "resolved",
            "confidence": "suspected",
            "resolution_basis": basis.as_str()
        }));
    }
    Ok(resolved)
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
    let observed_at = event_time(&envelope.occurred_at)?;
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
    let expected = record.expected_paths.iter().cloned().collect();
    !changed_set.is_empty() && changed_set.is_subset(&expected)
}

fn matched_paths_cover_observed(
    record: &ExpectedWriteRecord,
    changed_set: &BTreeSet<String>,
) -> bool {
    let expected = record
        .matched_paths
        .as_ref()
        .map(|paths| paths.iter().cloned().collect())
        .unwrap_or_default();
    !changed_set.is_empty() && changed_set.is_subset(&expected)
}

fn matched_expected_write_json(
    record: &ExpectedWriteRecord,
    changed: &[String],
    repository_identity: Option<&Value>,
) -> Value {
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
        "write_ticket_ids": record.write_ticket_ids,
        "repository_identity": repository_identity
    })
}

fn ticket_backed_observation_json(
    ticket: &ActiveWriteTicketSummary,
    changed: &[String],
    repository_identity: Option<&Value>,
) -> Value {
    json!({
        "status": "ticket_backed",
        "ticket_backed": true,
        "write_ticket_id": ticket.write_ticket_id.clone(),
        "write_ticket_ids": [ticket.write_ticket_id.clone()],
        "observed_paths": changed,
        "change_unit_id": ticket.change_unit_id.clone(),
        "allowed_path_prefixes": ticket.intended_paths.clone(),
        "denied_path_prefixes": ticket.denied_paths.clone(),
        "idle_expires_at": ticket.idle_expires_at.clone(),
        "repository_identity": repository_identity
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_suppression_retains_every_observed_path_and_machine_diagnostic() {
        let outcome = SuppressionOutcome::Unavailable {
            remaining_paths: vec!["src/a.rs".to_owned(), "src/b.rs".to_owned()],
            reason: SuppressionUnavailableReason::StoredEventCorrupt,
            scan_budget: POST_TOOL_CORRELATION_EVENT_LIMIT,
            observed_count: 7,
        };

        assert_eq!(
            outcome.remaining_paths(),
            ["src/a.rs".to_owned(), "src/b.rs".to_owned()]
        );
        assert!(outcome.suppressions().is_empty());
        assert!(outcome.is_unavailable());
        assert_eq!(outcome.to_json()["status"], "unavailable");
        assert_eq!(outcome.to_json()["reason"], "stored_event_corrupt");
        assert_eq!(
            outcome.to_json()["scan_budget"],
            POST_TOOL_CORRELATION_EVENT_LIMIT
        );
        assert_eq!(outcome.to_json()["observed_count"], 7);
        assert_eq!(
            outcome
                .diagnostic_json()
                .expect("unavailable has diagnostic")["severity"],
            "warning"
        );
    }

    #[test]
    fn applied_suppression_projects_remaining_paths_and_suppressions() {
        let outcome = SuppressionOutcome::Applied {
            remaining_paths: vec!["src/remaining.rs".to_owned()],
            suppressions: vec![json!({"status": "recorded_identity_unchanged"})],
        };

        assert_eq!(outcome.status(), "applied");
        assert!(!outcome.is_unavailable());
        assert_eq!(outcome.to_json()["remaining_paths"][0], "src/remaining.rs");
        assert_eq!(
            outcome.to_json()["suppressions"][0]["status"],
            "recorded_identity_unchanged"
        );
        assert!(outcome.diagnostic_json().is_none());
    }

    #[test]
    fn bounded_event_overflow_has_stable_reason_budget_and_probe_count() {
        let failure = correlation_event_query_failure(StoreError::InvalidInput {
            detail: format!(
                "post-tool correlation window exceeds the bounded event limit of {}",
                POST_TOOL_CORRELATION_EVENT_LIMIT
            ),
        });

        assert_eq!(
            failure.reason,
            SuppressionUnavailableReason::EventWindowExceeded
        );
        assert_eq!(
            failure.observed_count,
            POST_TOOL_CORRELATION_EVENT_LIMIT + 1
        );
    }

    #[test]
    fn malformed_correlation_arrays_are_not_silently_ignored() {
        assert!(durable_correlation_entries(&json!({
            "matched_expected_writes": {},
            "ticket_backed_observations": []
        }))
        .is_err());
    }
}
