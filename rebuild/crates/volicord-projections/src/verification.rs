//! Independent read-side facts, never a second verification authority. Free text
//! stays quoted: labels, assertion IDs, environment and repair causality are not
//! reconstructed from a Checkpoint outcome or a successful process exit.
use crate::WorkStateObservation;
use serde::Serialize;
use volicord_context::{CanonicalReadBasis, CommandTermination, SourcePayload, VerificationState};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "state", content = "value", rename_all = "snake_case")]
pub enum EvidenceDetail<T> {
    Observed(T),
    NotObserved,
    Unavailable,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportedCheckState {
    NotRun,
    Partial,
    Passed,
    Failed,
}
impl From<VerificationState> for ReportedCheckState {
    fn from(value: VerificationState) -> Self {
        match value {
            VerificationState::NotRun => Self::NotRun,
            VerificationState::Partial => Self::Partial,
            VerificationState::Passed => Self::Passed,
            VerificationState::Failed => Self::Failed,
        }
    }
}
impl ReportedCheckState {
    pub fn label(self, locale: crate::FixedLocale) -> &'static str {
        crate::verification_state_label(
            match self {
                Self::NotRun => VerificationState::NotRun,
                Self::Partial => VerificationState::Partial,
                Self::Passed => VerificationState::Passed,
                Self::Failed => VerificationState::Failed,
            },
            locale,
        )
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationApplicability {
    HistoricalAfterRecordedChange,
    Unknown,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RetestComparison {
    Unresolved,
}

/// Original vector position within this exact Checkpoint revision, NOT an
/// authoritative test ID or a comparable target retained by the old report.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct VerificationReference {
    pub checkpoint_id: String,
    pub checkpoint_revision: u64,
    pub verification_index: usize,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct VerificationReading {
    pub original: VerificationReference,
    pub reported_at_unix_micros: i64,
    pub reported_state: ReportedCheckState,
    pub original_outcome: Option<String>,
    pub source_id: Option<String>,
    pub intended_check: EvidenceDetail<String>,
    pub assertion_result: EvidenceDetail<String>,
    pub applicability: VerificationApplicability,
    pub later_changed_checkpoint_ids: Vec<String>,
    pub retest_comparison: RetestComparison,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionTermination {
    SpawnFailed,
    Exited,
    Signaled,
    Indeterminate,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandBodyState {
    NotRetained,
    Available,
    Deleted,
    Expired,
    Changed,
    Invalidated,
    Unavailable,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StreamCapture {
    pub observed_bytes: u64,
    pub omitted_bytes: Option<u64>,
    pub representation: StreamRepresentation,
    pub capture_completeness: CaptureCompleteness,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureCompleteness {
    Complete,
    Incomplete,
    Unobserved,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceEvidenceState {
    Available,
    Current,
    Stale,
    Unavailable,
    Unknown,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExecutionSourceStatus {
    pub availability: SourceEvidenceState,
    pub freshness: SourceEvidenceState,
    pub snapshot_basis: Option<String>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamRepresentation {
    WholeStream,
    BoundedExcerpt,
    Unavailable,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionTargetState {
    SelectedAnalysisOnlyNotRechecked,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ScopedCommandReading {
    pub operation_id: String,
    pub work_revision: u64,
    pub analysis_snapshot_id: String,
    pub repository_snapshot_id: String,
    pub repository_source_id: String,
    pub target_state: ExecutionTargetState,
    pub started_at_unix_micros: i64,
    pub ended_at_unix_micros: i64,
    pub retained_until_unix_micros: i64,
    pub signal_number: Option<i64>,
    pub timeout_detected: bool,
    pub cancellation_requested: bool,
    pub stdout: StreamCapture,
    pub stderr: StreamCapture,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExecutionReading {
    pub source_id: String,
    pub source_revision: u64,
    pub source_status: ExecutionSourceStatus,
    /// Presentation only; never a parsed test name or correlation key.
    pub command_label: String,
    pub invocation_fingerprint: String,
    pub recorded_at_unix_micros: i64,
    pub exit_code: Option<i32>,
    pub termination: ExecutionTermination,
    pub body_state: CommandBodyState,
    pub observation: EvidenceDetail<ScopedCommandReading>,
    pub observation_fingerprint: Option<String>,
    pub environment: EvidenceDetail<String>,
    pub assertion_result: EvidenceDetail<String>,
    pub checkpoint_reports: Vec<VerificationReference>,
}
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct WorkVerificationReport {
    pub checks: Vec<VerificationReading>,
    pub executions: Vec<ExecutionReading>,
}

pub(crate) fn verification_report(
    canonical: &CanonicalReadBasis,
    states: &[WorkStateObservation],
) -> WorkVerificationReport {
    let checks: Vec<_> = states
        .iter()
        .flat_map(|s| {
            s.verification
                .iter()
                .enumerate()
                .map(|(index, fact)| VerificationReading {
                    original: VerificationReference {
                        checkpoint_id: s.checkpoint_id.to_string(),
                        checkpoint_revision: s.checkpoint_revision,
                        verification_index: index,
                    },
                    reported_at_unix_micros: s.observed_at.as_unix_micros(),
                    reported_state: fact.state.into(),
                    original_outcome: fact.outcome.clone(),
                    source_id: fact.source_id.map(|id| id.to_string()),
                    intended_check: EvidenceDetail::NotObserved,
                    assertion_result: EvidenceDetail::NotObserved,
                    applicability: if s.later_changed_checkpoint_ids.is_empty() {
                        VerificationApplicability::Unknown
                    } else {
                        VerificationApplicability::HistoricalAfterRecordedChange
                    },
                    later_changed_checkpoint_ids: s
                        .later_changed_checkpoint_ids
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                    retest_comparison: RetestComparison::Unresolved,
                })
        })
        .collect();
    let mut executions = Vec::new();
    for id in checks
        .iter()
        .filter_map(|c| c.source_id.as_deref())
        .collect::<std::collections::BTreeSet<_>>()
    {
        if let Some(execution) = execution_reading(canonical, id, &checks) {
            executions.push(execution);
        }
    }
    WorkVerificationReport { checks, executions }
}

pub fn execution_reading(
    canonical: &CanonicalReadBasis,
    id: &str,
    checks: &[VerificationReading],
) -> Option<ExecutionReading> {
    let source = canonical
        .sources
        .iter()
        .find(|s| s.source.id.to_string() == id)?;
    let SourcePayload::CommandExecution {
        command_label,
        invocation_fingerprint,
        outcome,
    } = &source.source.payload
    else {
        return None;
    };
    Some(ExecutionReading {
        source_id: id.into(),
        source_revision: 1,
        source_status: ExecutionSourceStatus {
            availability: match source.availability {
                volicord_context::Availability::Available => SourceEvidenceState::Available,
                volicord_context::Availability::Stale => SourceEvidenceState::Stale,
                volicord_context::Availability::Unavailable => SourceEvidenceState::Unavailable,
                volicord_context::Availability::Unknown => SourceEvidenceState::Unknown,
            },
            freshness: match source.freshness {
                volicord_context::SourceFreshness::Current => SourceEvidenceState::Current,
                volicord_context::SourceFreshness::Stale => SourceEvidenceState::Stale,
                volicord_context::SourceFreshness::Unavailable => SourceEvidenceState::Unavailable,
                volicord_context::SourceFreshness::Unknown => SourceEvidenceState::Unknown,
            },
            snapshot_basis: source.snapshot_basis.clone(),
        },
        command_label: command_label.clone(),
        invocation_fingerprint: invocation_fingerprint.clone(),
        recorded_at_unix_micros: source.source.recorded_at.as_unix_micros(),
        exit_code: outcome.exit_code,
        termination: match outcome.termination {
            CommandTermination::Exited => ExecutionTermination::Exited,
            CommandTermination::SpawnFailed => ExecutionTermination::SpawnFailed,
            CommandTermination::Signaled => ExecutionTermination::Signaled,
            CommandTermination::Indeterminate => ExecutionTermination::Indeterminate,
        },
        body_state: CommandBodyState::NotRetained,
        observation: EvidenceDetail::NotObserved,
        observation_fingerprint: None,
        environment: EvidenceDetail::NotObserved,
        assertion_result: EvidenceDetail::NotObserved,
        checkpoint_reports: checks
            .iter()
            .filter(|c| c.source_id.as_deref() == Some(id))
            .map(|c| c.original.clone())
            .collect(),
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkVerificationBasis {
    pub work_item_id: volicord_context::ContextItemId,
    pub source_id: String,
    pub observation_fingerprint: String,
}
pub fn work_verification_publication_basis(
    projection: &crate::ProjectProjection,
) -> Vec<WorkVerificationBasis> {
    projection
        .selected_work
        .iter()
        .chain(&projection.work_history)
        .chain(&projection.work_overview.current.items)
        .chain(&projection.work_overview.completed.items)
        .chain(&projection.work_overview.remaining.items)
        .chain(&projection.work_overview.next_steps.items)
        .flat_map(|w| {
            w.reading
                .verification_report
                .executions
                .iter()
                .filter_map(move |e| {
                    e.observation_fingerprint
                        .as_ref()
                        .map(|f| (w.work_item_id, e.source_id.clone(), f.clone()))
                })
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(
            |(work_item_id, source_id, observation_fingerprint)| WorkVerificationBasis {
                work_item_id,
                source_id,
                observation_fingerprint,
            },
        )
        .collect()
}
