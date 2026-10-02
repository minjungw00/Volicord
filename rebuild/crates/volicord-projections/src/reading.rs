//! Deterministic reading representations. Original text and independent facts
//! remain inspectable; quotations never claim semantic comprehension.
use crate::{BriefDecision, UnderstandingWork, UnderstandingWorkState};
use std::collections::BTreeSet;
use volicord_context::{
    CanonicalReadBasis, CanonicalRecordKind, CheckpointId, ContextItemId, ContextItemRole,
    DecisionId, DecisionWorkScope, SourceFreshness, SourceId, TimestampMicros, UserAcceptanceFact,
    UserReviewFact, VerificationFact, WorkState,
};
use volicord_repository_intelligence::{AnalysisSnapshotId, RepositorySnapshotId};

pub const READING_TEXT_CHARACTER_LIMIT: usize = 384;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReadingRecord {
    ContextItem(ContextItemId),
    Checkpoint(CheckpointId),
    Decision(DecisionId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadingBasis {
    pub record: ReadingRecord,
    pub revision: u64,
    pub available_revisions: Vec<u64>,
    pub field: String,
    pub source_basis: Vec<SourceId>,
    pub source_status: Vec<ReadingSourceStatus>,
    pub analysis_snapshot_basis: Vec<AnalysisSnapshotId>,
    pub repository_snapshot_basis: Vec<RepositorySnapshotId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadingSourceStatus {
    pub source_id: SourceId,
    pub availability: Option<volicord_context::Availability>,
    pub freshness: SourceFreshness,
    pub snapshot_basis: Option<String>,
    pub actor: Option<volicord_context::Principal>,
    pub observer: Option<volicord_context::Principal>,
    pub recorded_at: Option<TimestampMicros>,
}

pub(crate) fn reading_sources(
    canonical: &CanonicalReadBasis,
    ids: &[SourceId],
) -> Vec<ReadingSourceStatus> {
    ids.iter()
        .map(|id| {
            let source = canonical.sources.iter().find(|s| s.source.id == *id);
            ReadingSourceStatus {
                source_id: *id,
                availability: source.map(|s| s.availability),
                freshness: source.map_or(SourceFreshness::Unknown, |s| s.freshness),
                snapshot_basis: source.and_then(|s| s.snapshot_basis.clone()),
                actor: source.map(|s| s.source.actor.clone()),
                observer: source.and_then(|s| s.source.observer.clone()),
                recorded_at: source.map(|s| s.source.recorded_at),
            }
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadingRepresentation {
    OriginalQuotation,
    Excerpt,
    DeterministicFacts,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadingAvailability {
    NotRequested,
    Unknown,
    Available,
    Degraded,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadingText {
    pub original_text: Option<String>,
    pub display_english: String,
    pub display_korean: String,
    pub representation: ReadingRepresentation,
    pub availability: ReadingAvailability,
    pub basis: ReadingBasis,
    pub omitted_utf8_bytes: usize,
    pub omitted_characters: usize,
    /// Quoted original text is not a translated or semantically realized summary.
    pub semantic_summary_available: bool,
    pub original_language_preserved: bool,
    pub gaps: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkStateObservation {
    pub checkpoint_id: CheckpointId,
    pub checkpoint_revision: u64,
    pub observed_at: TimestampMicros,
    pub work_state: WorkState,
    pub work_source_basis: Vec<SourceId>,
    pub verification: Vec<VerificationFact>,
    pub user_review: UserReviewFact,
    pub user_acceptance: UserAcceptanceFact,
    /// Conservatively disclose chronology, without claiming test coverage.
    pub later_changed_checkpoint_ids: Vec<CheckpointId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkCodeGap {
    NoSeeds,
    NoMatchingCode,
    AnalysisUnavailable,
    AnalysisNotCurrent,
}

impl WorkCodeGap {
    pub const fn english(self) -> &'static str {
        match self {
            Self::NoSeeds => "No path/component seeds are recorded; Source/code links have not been evaluated",
            Self::NoMatchingCode => "No snapshot-bound code matches this Work's seeds",
            Self::AnalysisUnavailable => "Analysis unavailable; canonical Work remains readable",
            Self::AnalysisNotCurrent => "Analysis is stale or current repository observation is unavailable; historical topology remains inspectable",
        }
    }
    pub const fn korean(self) -> &'static str {
        match self {
            Self::NoSeeds => "경로·컴포넌트 근거가 기록되지 않았으며 Source·코드 연결은 아직 평가하지 않았습니다",
            Self::NoMatchingCode => "이 작업의 근거와 일치하는 snapshot 코드가 없습니다",
            Self::AnalysisUnavailable => "분석을 사용할 수 없습니다. Canonical 작업은 계속 읽을 수 있습니다",
            Self::AnalysisNotCurrent => "분석이 오래되었거나 현재 저장소 관찰을 사용할 수 없습니다. 과거 topology는 확인할 수 있습니다",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkReading {
    pub goal: ReadingText,
    pub changes: Vec<ReadingText>,
    pub next_step: ReadingText,
    pub status: ReadingText,
    pub states: Vec<WorkStateObservation>,
    pub code_gap: Option<WorkCodeGap>,
    pub code_availability: ReadingAvailability,
    pub code_freshness: Vec<volicord_repository_intelligence::FreshnessBasis>,
    pub code_source_basis: Vec<SourceId>,
    pub analysis_snapshot_basis: Vec<AnalysisSnapshotId>,
    pub repository_snapshot_basis: Vec<RepositorySnapshotId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionReading {
    pub user_rationale: ReadingText,
    pub recommendation_rationale: ReadingText,
    pub chosen_alternative_key: Option<String>,
    pub recommended_alternative_key: Option<String>,
    pub work_scope: DecisionWorkScope,
}

pub(crate) fn reading_basis(
    canonical: &CanonicalReadBasis,
    record: ReadingRecord,
    revision: u64,
    field: &str,
    sources: Vec<SourceId>,
) -> ReadingBasis {
    let (kind, identity) = match record {
        ReadingRecord::ContextItem(id) => (CanonicalRecordKind::ContextItem, id.to_string()),
        ReadingRecord::Checkpoint(id) => (CanonicalRecordKind::Checkpoint, id.to_string()),
        ReadingRecord::Decision(id) => (CanonicalRecordKind::Decision, id.to_string()),
    };
    ReadingBasis {
        record,
        revision,
        available_revisions: canonical
            .revisions
            .iter()
            .find(|r| r.record_kind == kind && r.record_identity == identity)
            .map_or_else(|| vec![revision], |r| r.revisions.clone()),
        field: field.into(),
        source_status: reading_sources(canonical, &sources),
        source_basis: sources,
        analysis_snapshot_basis: Vec::new(),
        repository_snapshot_basis: Vec::new(),
    }
}

pub(crate) fn quoted_reading(
    original: Option<&str>,
    basis: ReadingBasis,
    _canonical: Option<&CanonicalReadBasis>,
) -> ReadingText {
    let text = original.filter(|text| !text.trim().is_empty());
    let mut gaps = Vec::new();
    let degraded = basis.source_status.iter().any(|source| {
        source.freshness != SourceFreshness::Current
            || source.availability != Some(volicord_context::Availability::Available)
    });
    if degraded {
        gaps.push("Some Source basis is stale, unavailable or unknown".into());
    }
    let Some(text) = text else {
        gaps.push("No explanatory text is recorded; semantic summary unavailable".into());
        return ReadingText {
            original_text: original.map(str::to_owned),
            display_english: "Summary unavailable: no explanatory text is recorded".into(),
            display_korean: "설명 문장이 기록되지 않아 요약을 제공할 수 없습니다".into(),
            representation: ReadingRepresentation::Unavailable,
            availability: ReadingAvailability::Unavailable,
            basis,
            omitted_utf8_bytes: 0,
            omitted_characters: 0,
            semantic_summary_available: false,
            original_language_preserved: true,
            gaps,
        };
    };
    let prefix = text
        .chars()
        .take(READING_TEXT_CHARACTER_LIMIT)
        .collect::<String>();
    let omitted_characters = text.chars().count() - prefix.chars().count();
    let excerpt = omitted_characters > 0;
    let (display_english, display_korean) = if excerpt {
        (
            format!("Excerpt (original language): {prefix}"),
            format!("발췌 (원문 언어): {prefix}"),
        )
    } else {
        (text.to_owned(), text.to_owned())
    };
    if excerpt {
        gaps.push("Excerpt only; interpret the original record for its full meaning".into());
    }
    ReadingText {
        original_text: Some(text.to_owned()),
        display_english,
        display_korean,
        representation: if excerpt {
            ReadingRepresentation::Excerpt
        } else {
            ReadingRepresentation::OriginalQuotation
        },
        availability: if excerpt || degraded {
            ReadingAvailability::Degraded
        } else {
            ReadingAvailability::Available
        },
        basis,
        omitted_utf8_bytes: text.len() - prefix.len(),
        omitted_characters,
        semantic_summary_available: false,
        original_language_preserved: true,
        gaps,
    }
}

pub(crate) fn decision_reading(decision: &BriefDecision) -> DecisionReading {
    let basis =
        |field: &str, sources: Vec<SourceId>, status: Vec<ReadingSourceStatus>| ReadingBasis {
            record: ReadingRecord::Decision(decision.decision_id),
            revision: decision.revision,
            available_revisions: decision.available_revisions.clone(),
            field: field.into(),
            source_basis: sources,
            source_status: status,
            analysis_snapshot_basis: Vec::new(),
            repository_snapshot_basis: Vec::new(),
        };
    DecisionReading {
        user_rationale: quoted_reading(
            decision.user_rationale.as_deref(),
            basis(
                "user_rationale",
                decision.user_source_basis.clone(),
                decision.user_source_status.clone(),
            ),
            None,
        ),
        recommendation_rationale: quoted_reading(
            Some(&decision.recommendation_rationale),
            basis(
                "displayed_recommendation.rationale",
                decision.recommendation_source_basis.clone(),
                decision.recommendation_source_status.clone(),
            ),
            None,
        ),
        chosen_alternative_key: decision.chosen_alternative_key.clone(),
        recommended_alternative_key: decision.recommended_alternative_key.clone(),
        work_scope: decision.work_scope,
    }
}

/// Aggregate complete canonical history before any projection/list/transport bound.
pub(crate) fn derive_work_history(canonical: &CanonicalReadBasis) -> Vec<UnderstandingWork> {
    canonical.context_items.iter().filter(|goal|
        goal.project_id == canonical.project.id && goal.role == ContextItemRole::Goal)
        .map(|goal| {
            let mut checkpoints = canonical.checkpoint_history.iter().chain(canonical.latest_checkpoint.iter())
                .filter(|cp| cp.project_id == canonical.project.id && cp.work_item_id == Some(goal.id))
                .collect::<Vec<_>>();
            checkpoints.sort_by_key(|cp| (cp.recorded_at, cp.id));
            checkpoints.dedup_by_key(|cp| cp.id);
            let latest = checkpoints.last().copied();
            let mut decisions = checkpoints.iter().flat_map(|cp| cp.applied_decisions.iter().copied()).collect::<BTreeSet<_>>();
            decisions.extend(canonical.active_decisions.iter().chain(&canonical.superseded_decisions)
                .filter(|d| d.decision.work_scope == DecisionWorkScope::WorkItem(goal.id))
                .map(|d| d.decision.id));
            let source_basis = goal.source_basis.iter().copied().chain(checkpoints.iter().flat_map(|cp|
                cp.source_basis.iter().chain(&cp.changed_source_basis).copied()
                    .chain(cp.verification.iter().filter_map(|v| v.source_id))
                    .chain(cp.user_review.source_id).chain(cp.user_acceptance.source_id)))
                .collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>();
            let changed_paths = checkpoints.iter().flat_map(|cp| cp.changed_paths.iter().cloned())
                .collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>();
            let states = checkpoints.iter().enumerate().map(|(index, cp)| WorkStateObservation {
                checkpoint_id: cp.id, checkpoint_revision: cp.revision, observed_at: cp.recorded_at,
                work_state: cp.work_state, work_source_basis: cp.source_basis.clone(),
                verification: cp.verification.clone(), user_review: cp.user_review.clone(), user_acceptance: cp.user_acceptance.clone(),
                later_changed_checkpoint_ids: checkpoints[index + 1..].iter()
                    .filter(|later| !later.changed_paths.is_empty() || !later.changed_source_basis.is_empty())
                    .map(|later| later.id).collect(),
            }).collect();
            let goal_basis = reading_basis(canonical, ReadingRecord::ContextItem(goal.id), goal.revision, "statement", goal.source_basis.clone());
            let mut status_basis = goal_basis.clone();
            status_basis.field = "role".into();
            if let Some(cp) = latest {
                status_basis = reading_basis(canonical, ReadingRecord::Checkpoint(cp.id), cp.revision, "work_state", cp.source_basis.clone());
            }
            let state = latest.map_or(UnderstandingWorkState::Open, |cp| cp.work_state.into());
            let mut status = ReadingText {
                original_text: None,
                display_english: format!("Recorded work state: {state:?}. Verification, user review and acceptance are separate observations."),
                display_korean: format!("기록된 작업 상태: {}. 검증, 사용자 검토와 수락은 독립적인 관찰입니다.", match state {
                    UnderstandingWorkState::Open => "열림", UnderstandingWorkState::InProgress => "진행 중", UnderstandingWorkState::Paused => "일시 중지",
                    UnderstandingWorkState::Completed => "완료", UnderstandingWorkState::Abandoned => "중단", UnderstandingWorkState::Superseded => "대체됨",
                }),
                representation: ReadingRepresentation::DeterministicFacts,
                availability: ReadingAvailability::Available, basis: status_basis,
                omitted_utf8_bytes: 0, omitted_characters: 0, semantic_summary_available: true,
                original_language_preserved: false, gaps: Vec::new(),
            };
            let changes = checkpoints.iter().map(|cp| quoted_reading(cp.state_change.as_deref(),
                reading_basis(canonical, ReadingRecord::Checkpoint(cp.id), cp.revision, "state_change", cp.source_basis.clone()), Some(canonical))).collect();
            if latest.is_none() {
                status.display_english = "Goal-only Work is open; no Checkpoint work state is recorded".into();
                status.display_korean = "Goal만 있는 작업은 열림으로 읽습니다. Checkpoint 작업 상태는 기록되지 않았습니다".into();
            }
            let mut next_step = latest.map_or_else(|| quoted_reading(None, goal_basis.clone(), Some(canonical)), |cp|
                quoted_reading(Some(&cp.next_step), reading_basis(canonical, ReadingRecord::Checkpoint(cp.id), cp.revision, "next_step", cp.source_basis.clone()), Some(canonical)));
            if latest.is_none() {
                next_step.basis.field = "role".into();
                next_step.gaps.push("No same-Work Checkpoint records a next step".into());
            }
            let has_scope = !goal.applicability.paths.is_empty() || !goal.applicability.components.is_empty();
            UnderstandingWork {
                work_item_id: goal.id, title: goal.statement.clone(), state,
                checkpoint_ids: checkpoints.iter().map(|cp| cp.id).collect(), decision_ids: decisions.iter().copied().collect(),
                meaningful_changes: checkpoints.iter().filter_map(|cp| cp.state_change.clone()).collect(), changed_paths: changed_paths.clone(),
                changed_components: canonical.active_decisions.iter().chain(&canonical.superseded_decisions)
                    .filter(|d| decisions.contains(&d.decision.id)).flat_map(|d| d.decision.applicability.components.iter().cloned())
                    .collect::<BTreeSet<_>>().into_iter().collect(),
                verification: checkpoints.iter().flat_map(|cp| cp.verification.iter().cloned()).collect(),
                next_step: latest.map(|cp| cp.next_step.clone()),
                open_question_ids: latest.into_iter().flat_map(|cp| cp.open_questions.iter().map(|q| q.question_id))
                    .filter(|id| canonical.active_questions.iter().any(|q| q.id == *id)).collect(),
                source_basis,
                reading: WorkReading {
                    goal: quoted_reading(Some(&goal.statement), goal_basis, Some(canonical)), changes, next_step, status, states,
                    code_availability: ReadingAvailability::Unknown,
                    code_freshness: Vec::new(), code_source_basis: Vec::new(),
                    analysis_snapshot_basis: Vec::new(), repository_snapshot_basis: Vec::new(),
                    code_gap: (changed_paths.is_empty() && !has_scope).then_some(WorkCodeGap::NoSeeds),
                },
            }
        }).collect()
}

/// Shared full-history selection used by topology and exact Work reading.
pub(crate) fn scope_to_work(
    canonical: &CanonicalReadBasis,
    work: ContextItemId,
) -> CanonicalReadBasis {
    let mut scoped = canonical.clone();
    scoped
        .checkpoint_history
        .retain(|cp| cp.project_id == canonical.project.id && cp.work_item_id == Some(work));
    if let Some(cp) = &canonical.latest_checkpoint {
        if cp.work_item_id == Some(work)
            && !scoped
                .checkpoint_history
                .iter()
                .any(|item| item.id == cp.id)
        {
            scoped.checkpoint_history.push(cp.clone());
        }
    }
    scoped
        .checkpoint_history
        .sort_by_key(|cp| (cp.recorded_at, cp.id));
    scoped.latest_checkpoint = scoped.checkpoint_history.last().cloned();
    let applied = scoped
        .checkpoint_history
        .iter()
        .flat_map(|cp| cp.applied_decisions.iter().copied())
        .collect::<BTreeSet<_>>();
    let relevant = |d: &volicord_context::DecisionLifecycle| {
        d.decision.work_scope == DecisionWorkScope::WorkItem(work)
            || applied.contains(&d.decision.id)
    };
    scoped.active_decisions.retain(relevant);
    scoped.superseded_decisions.retain(relevant);
    scoped
        .context_items
        .retain(|item| item.role != ContextItemRole::Goal || item.id == work);
    let questions = scoped
        .checkpoint_history
        .iter()
        .flat_map(|cp| cp.open_questions.iter().map(|q| q.question_id))
        .chain(
            scoped
                .active_decisions
                .iter()
                .chain(&scoped.superseded_decisions)
                .map(|d| d.decision.question_id),
        )
        .collect::<BTreeSet<_>>();
    scoped
        .active_questions
        .retain(|q| questions.contains(&q.id));
    scoped
        .terminal_question_history
        .retain(|q| questions.contains(&q.id));
    scoped
}

pub(crate) fn derive_unresolved_grouping(
    canonical: &CanonicalReadBasis,
) -> Vec<crate::UnresolvedWorkGrouping> {
    let goals = canonical
        .context_items
        .iter()
        .filter(|item| item.role == ContextItemRole::Goal)
        .map(|item| item.id)
        .collect::<BTreeSet<_>>();
    let mut unresolved = canonical
        .active_decisions
        .iter()
        .chain(&canonical.superseded_decisions)
        .filter(|lifecycle| lifecycle.decision.work_scope == DecisionWorkScope::Unresolved)
        .map(|lifecycle| crate::UnresolvedWorkGrouping {
            record_kind: "decision",
            identity: lifecycle.decision.id.to_string(),
            reason: "Decision has no explicit Project-wide or Work Item scope".into(),
        })
        .collect::<Vec<_>>();
    unresolved.extend(
        canonical
            .checkpoint_history
            .iter()
            .chain(canonical.latest_checkpoint.iter())
            .filter(|cp| cp.work_item_id.is_none_or(|id| !goals.contains(&id)))
            .map(|cp| crate::UnresolvedWorkGrouping {
                record_kind: "checkpoint",
                identity: cp.id.to_string(),
                reason: "Checkpoint has no available explicit Goal Work identity".into(),
            }),
    );
    unresolved.sort_by(|left, right| {
        (left.record_kind, &left.identity).cmp(&(right.record_kind, &right.identity))
    });
    unresolved.dedup();
    unresolved
}
