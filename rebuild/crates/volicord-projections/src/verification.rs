//! Independent read-side facts, never a second verification authority. Free text
//! stays quoted: labels, assertion IDs, environment and repair causality are not
//! reconstructed from a Checkpoint outcome or a successful process exit.
use crate::WorkStateObservation;
use serde::Serialize;
use volicord_context::{
    AssertionState, CanonicalReadBasis, CheckTargetRelation, CommandTermination, ScopedCheckRun,
    SourcePayload, VerificationState,
};

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
    CapturedArtifactsAtExecution,
    Unknown,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum RetestComparison {
    Unresolved,
    Comparable {
        previous: VerificationReference,
        source_id: String,
        target_relation: CheckTargetRelation,
        earlier_assertion: AssertionState,
        later_assertion: AssertionState,
    },
    NotComparable {
        previous: VerificationReference,
        reason: ComparisonLimit,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonLimit {
    PriorRunUnavailable,
    CheckIdentityOrPurposeChanged,
    InvocationChanged,
    CoverageMissingOrDifferent,
    EnvironmentMissingOrDifferent,
    TargetScopeChanged,
    TargetChangedDuringRun,
    TargetRelationContradicted,
    AssertionNotObserved,
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
    /// Original canonical report basis survives disposal of execution output.
    pub scoped_run: EvidenceDetail<ScopedCheckRun>,
    pub declared_coverage: EvidenceDetail<String>,
    pub declared_environment: EvidenceDetail<String>,
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
    DeclaredArtifactsSampled,
    DeclaredArtifactsChanged,
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
    pub capture_fingerprint: Option<String>,
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
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorkVerificationReport {
    pub checks: Vec<VerificationReading>,
    pub executions: Vec<ExecutionReading>,
}

// Read-time metadata, not new canonical totals or a stored second count authority.
impl Serialize for WorkVerificationReport {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        #[derive(Serialize)]
        struct Counts {
            total: usize,
            passed: usize,
            failed: usize,
            partial: usize,
            not_run: usize,
        }
        let mut value = serializer.serialize_struct("WorkVerificationReport", 6)?;
        value.serialize_field("report_count_unit", "checkpoint_verification_report")?;
        value.serialize_field("report_count_scope", "selected_work_checkpoint_history")?;
        value.serialize_field("reported_test_count_basis", "original_report_text_only")?;
        value.serialize_field(
            "report_counts",
            &Counts {
                total: self.checks.len(),
                passed: self.report_count(ReportedCheckState::Passed),
                failed: self.report_count(ReportedCheckState::Failed),
                partial: self.report_count(ReportedCheckState::Partial),
                not_run: self.report_count(ReportedCheckState::NotRun),
            },
        )?;
        value.serialize_field("checks", &self.checks)?;
        value.serialize_field("executions", &self.executions)?;
        value.end()
    }
}

impl WorkVerificationReport {
    /// Count canonical report entries across the selected Work's Checkpoint
    /// history. Repeated Source references remain distinct reports. Neither
    /// free-text outcomes nor execution identities supply a test count.
    pub fn report_count(&self, state: ReportedCheckState) -> usize {
        self.checks
            .iter()
            .filter(|c| c.reported_state == state)
            .count()
    }

    /// Shared by public answers and Viewer; no parsing or inferred test totals.
    pub fn report_summary(&self, locale: crate::FixedLocale) -> String {
        let counts = [
            ReportedCheckState::Passed,
            ReportedCheckState::Failed,
            ReportedCheckState::Partial,
            ReportedCheckState::NotRun,
        ]
        .into_iter()
        .filter_map(|state| {
            let count = self.report_count(state);
            (count > 0).then(|| match locale {
                crate::FixedLocale::English => format!(
                    "{count} {} {}",
                    state.label(locale),
                    if count == 1 { "report" } else { "reports" }
                ),
                crate::FixedLocale::Korean => format!("{} 보고 {count}건", state.label(locale)),
            })
        })
        .collect::<Vec<_>>()
        .join(" · ");
        match locale {
            crate::FixedLocale::English if counts.is_empty() =>
                "Verification reports (Work Checkpoint history): none recorded.".into(),
            crate::FixedLocale::Korean if counts.is_empty() =>
                "검증 보고 (Work Checkpoint 이력): 기록 없음.".into(),
            crate::FixedLocale::English => format!(
                "Verification reports (Work Checkpoint history): {counts}. Reported test counts: see individual original reports; no test total is inferred."),
            crate::FixedLocale::Korean => format!(
                "검증 보고 (Work Checkpoint 이력): {counts}. 보고된 테스트 수는 각 원래 보고문에서 확인하며 테스트 총수를 추론하지 않습니다."),
        }
    }
}

pub(crate) fn verification_report(
    canonical: &CanonicalReadBasis,
    states: &[WorkStateObservation],
) -> WorkVerificationReport {
    let mut checks: Vec<_> = states
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
                    intended_check: fact.scoped_run.as_ref().map_or(
                        EvidenceDetail::NotObserved,
                        |run| {
                            EvidenceDetail::Observed(format!(
                                "{}: {}",
                                run.declaration.key, run.declaration.purpose
                            ))
                        },
                    ),
                    assertion_result: fact
                        .scoped_run
                        .as_ref()
                        .filter(|r| r.assertion_state != AssertionState::NotObserved)
                        .map_or(EvidenceDetail::NotObserved, |run| {
                            EvidenceDetail::Observed(format!(
                                "Operator reported {:?}: {}",
                                run.assertion_state, run.declaration.assertion
                            ))
                        }),
                    scoped_run: fact
                        .scoped_run
                        .clone()
                        .map_or(EvidenceDetail::NotObserved, EvidenceDetail::Observed),
                    declared_coverage: fact
                        .scoped_run
                        .as_ref()
                        .and_then(|r| r.declaration.coverage.clone())
                        .map_or(EvidenceDetail::NotObserved, EvidenceDetail::Observed),
                    declared_environment: fact
                        .scoped_run
                        .as_ref()
                        .and_then(|r| r.declaration.environment.clone())
                        .map_or(EvidenceDetail::NotObserved, EvidenceDetail::Observed),
                    applicability: if s.later_changed_checkpoint_ids.is_empty() {
                        if fact
                            .scoped_run
                            .as_ref()
                            .is_some_and(|r| r.target_unchanged_during_run)
                        {
                            VerificationApplicability::CapturedArtifactsAtExecution
                        } else {
                            VerificationApplicability::Unknown
                        }
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
    let comparisons = checks
        .iter()
        .map(|c| compare_retest(c, &checks))
        .collect::<Vec<_>>();
    for (check, comparison) in checks.iter_mut().zip(comparisons) {
        check.retest_comparison = comparison;
    }
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
        capture_fingerprint,
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
        capture_fingerprint: capture_fingerprint.clone(),
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

fn compare_retest(
    current: &VerificationReading,
    checks: &[VerificationReading],
) -> RetestComparison {
    let EvidenceDetail::Observed(run) = &current.scoped_run else {
        return RetestComparison::Unresolved;
    };
    let Some(link) = &run.retest else {
        return RetestComparison::Unresolved;
    };
    let reference = VerificationReference {
        checkpoint_id: link.checkpoint_id.to_string(),
        checkpoint_revision: link.checkpoint_revision,
        verification_index: link.verification_index,
    };
    let prior = checks
        .iter()
        .find(|c| {
            c.original == reference && c.source_id.as_deref() == Some(&link.source_id.to_string())
        })
        .and_then(|c| match &c.scoped_run {
            EvidenceDetail::Observed(r) => Some(r),
            _ => None,
        });
    let failure = |reason| RetestComparison::NotComparable {
        previous: reference.clone(),
        reason,
    };
    let Some(prior) = prior else {
        return failure(ComparisonLimit::PriorRunUnavailable);
    };
    if run.work_item_id != prior.work_item_id
        || run.work_revision != prior.work_revision
        || prior.ended_at_unix_micros >= run.started_at_unix_micros
    {
        return failure(ComparisonLimit::PriorRunUnavailable);
    }
    if run.declaration.key != prior.declaration.key
        || run.declaration.purpose != prior.declaration.purpose
        || run.declaration.assertion != prior.declaration.assertion
    {
        return failure(ComparisonLimit::CheckIdentityOrPurposeChanged);
    }
    if run.invocation_fingerprint != prior.invocation_fingerprint {
        return failure(ComparisonLimit::InvocationChanged);
    }
    if run.declaration.coverage.is_none() || run.declaration.coverage != prior.declaration.coverage
    {
        return failure(ComparisonLimit::CoverageMissingOrDifferent);
    }
    if run.declaration.environment.is_none()
        || run.declaration.environment != prior.declaration.environment
    {
        return failure(ComparisonLimit::EnvironmentMissingOrDifferent);
    }
    if run.declaration.target_paths != prior.declaration.target_paths {
        return failure(ComparisonLimit::TargetScopeChanged);
    }
    if !run.target_unchanged_during_run || !prior.target_unchanged_during_run {
        return failure(ComparisonLimit::TargetChangedDuringRun);
    }
    let equal = run.artifacts == prior.artifacts;
    if equal != (link.target_relation == CheckTargetRelation::SameArtifacts) {
        return failure(ComparisonLimit::TargetRelationContradicted);
    }
    if run.assertion_state == AssertionState::NotObserved
        || prior.assertion_state == AssertionState::NotObserved
    {
        return failure(ComparisonLimit::AssertionNotObserved);
    }
    RetestComparison::Comparable {
        previous: reference,
        source_id: link.source_id.to_string(),
        target_relation: link.target_relation,
        earlier_assertion: prior.assertion_state,
        later_assertion: run.assertion_state,
    }
}
