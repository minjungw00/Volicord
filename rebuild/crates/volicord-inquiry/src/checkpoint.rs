use std::collections::{BTreeMap, BTreeSet};
use volicord_context::{
    CanonicalReadBasis, Checkpoint, CheckpointDraft, CheckpointKind, DecisionId, OperationId,
    ProjectId, QuestionReference, SourceId, Store, UserAcceptanceFact, UserReviewFact,
    VerificationFact, WorkState,
};
use volicord_repository_intelligence::{
    AnalysisSnapshot, EntryKind, FreshnessState, InventoryClassification,
};

#[derive(Clone, Debug)]
pub struct RepositoryWorkBasis<'a> {
    pub baseline: &'a AnalysisSnapshot,
    pub current: &'a AnalysisSnapshot,
    /// Paths known dirty before this bounded work from an existing canonical
    /// Source/observation. They are evidence, not inferred ownership.
    pub pre_existing_dirty_paths: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChangeAttribution {
    Attributed {
        pre_existing_paths: Vec<String>,
        changed_paths: Vec<String>,
    },
    Ambiguous {
        pre_existing_paths: Vec<String>,
        attributable_paths: Vec<String>,
        ambiguous_paths: Vec<String>,
        reason: String,
    },
    Unavailable {
        pre_existing_paths: Vec<String>,
        reason: String,
    },
}

pub fn attribute_repository_changes(
    project_id: ProjectId,
    basis: &RepositoryWorkBasis<'_>,
) -> ChangeAttribution {
    let mut pre_existing = basis.pre_existing_dirty_paths.clone();
    pre_existing.sort();
    pre_existing.dedup();
    if basis.baseline.project.identity() != project_id
        || basis.current.project.identity() != project_id
        || basis.baseline.repository_source.identity() != basis.current.repository_source.identity()
    {
        return ChangeAttribution::Unavailable {
            pre_existing_paths: pre_existing,
            reason: "baseline and current Repository Intelligence basis do not identify the same Project Source"
                .to_owned(),
        };
    }
    if basis.baseline.freshness.state != FreshnessState::Current
        || basis.current.freshness.state != FreshnessState::Current
    {
        return ChangeAttribution::Unavailable {
            pre_existing_paths: pre_existing,
            reason: "change attribution requires current baseline and current observations"
                .to_owned(),
        };
    }
    let baseline = inventory_fingerprints(basis.baseline);
    let current = inventory_fingerprints(basis.current);
    let mut changed = baseline
        .keys()
        .chain(current.keys())
        .filter(|path| baseline.get(*path) != current.get(*path))
        .cloned()
        .collect::<BTreeSet<_>>();
    let pre_existing_set = pre_existing.iter().cloned().collect::<BTreeSet<_>>();
    let ambiguous = changed
        .intersection(&pre_existing_set)
        .cloned()
        .collect::<Vec<_>>();
    for path in &ambiguous {
        changed.remove(path);
    }
    let attributable = changed.into_iter().collect::<Vec<_>>();
    if ambiguous.is_empty() {
        ChangeAttribution::Attributed {
            pre_existing_paths: pre_existing,
            changed_paths: attributable,
        }
    } else {
        ChangeAttribution::Ambiguous {
            pre_existing_paths: pre_existing,
            attributable_paths: attributable,
            ambiguous_paths: ambiguous,
            reason: "a path dirty at the baseline changed again; current evidence cannot separate prior and bounded-work ownership"
                .to_owned(),
        }
    }
}

fn inventory_fingerprints(snapshot: &AnalysisSnapshot) -> BTreeMap<String, Option<String>> {
    snapshot
        .inventory
        .entries
        .iter()
        .filter(|entry| {
            entry.entry_kind == EntryKind::File
                && entry
                    .classifications
                    .contains(&InventoryClassification::Included)
        })
        .map(|entry| (entry.area.path.clone(), entry.content_sha256.clone()))
        .collect()
}

#[derive(Clone, Debug)]
pub struct CheckpointCandidate<'a> {
    pub project_id: ProjectId,
    pub kind: CheckpointKind,
    pub goal: String,
    pub work_state: WorkState,
    pub state_change: Option<String>,
    pub repository_work: Option<RepositoryWorkBasis<'a>>,
    pub supporting_sources: Vec<SourceId>,
    pub applied_decisions: Vec<DecisionId>,
    pub verification: Vec<VerificationFact>,
    pub user_review: UserReviewFact,
    pub user_acceptance: UserAcceptanceFact,
    pub known_limits: Vec<String>,
    pub non_goals: Vec<String>,
    pub next_step: String,
    pub handoff_to: Option<String>,
    pub status_only: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckpointRejection {
    WrongProject,
    StatusOnly,
    NoMeaningfulChange,
    MissingSourceBasis,
    SourceUnavailable,
    AmbiguousChangeAttribution,
    InvalidBoundary,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckpointEvaluation {
    Ready {
        draft: Box<CheckpointDraft>,
        attribution: ChangeAttribution,
    },
    Rejected {
        reason: CheckpointRejection,
        detail: String,
        attribution: Option<ChangeAttribution>,
    },
}

/// Converts a source-grounded Candidate into a Kernel draft, or reports why
/// ownership cannot be established. It does not persist the Checkpoint.
pub fn evaluate_checkpoint_candidate(
    canonical: &CanonicalReadBasis,
    candidate: CheckpointCandidate<'_>,
) -> CheckpointEvaluation {
    if canonical.project.id != candidate.project_id {
        return reject(
            CheckpointRejection::WrongProject,
            "canonical Project differs",
            None,
        );
    }
    if candidate.status_only {
        return reject(
            CheckpointRejection::StatusOnly,
            "status-only reads do not create canonical Checkpoints",
            None,
        );
    }
    let attribution = candidate
        .repository_work
        .as_ref()
        .map(|basis| attribute_repository_changes(candidate.project_id, basis));
    if let Some(ChangeAttribution::Unavailable { reason, .. }) = &attribution {
        return reject(
            CheckpointRejection::SourceUnavailable,
            reason.clone(),
            attribution,
        );
    }
    if let Some(ChangeAttribution::Ambiguous { reason, .. }) = &attribution {
        return reject(
            CheckpointRejection::AmbiguousChangeAttribution,
            reason.clone(),
            attribution,
        );
    }
    let changed_paths = match &attribution {
        Some(ChangeAttribution::Attributed { changed_paths, .. }) => changed_paths.clone(),
        _ => Vec::new(),
    };
    let current_repository_source = candidate
        .repository_work
        .as_ref()
        .map(|basis| basis.current.repository_source.identity());
    let mut source_basis = candidate.supporting_sources;
    if let Some(source) = current_repository_source {
        source_basis.push(source);
    }
    source_basis.sort_unstable();
    source_basis.dedup();
    let canonical_sources = canonical
        .sources
        .iter()
        .map(|basis| basis.source.id)
        .collect::<BTreeSet<_>>();
    if source_basis.is_empty()
        || source_basis
            .iter()
            .any(|source| !canonical_sources.contains(source))
    {
        return reject(
            CheckpointRejection::MissingSourceBasis,
            "Checkpoint support must use existing canonical Sources",
            attribution,
        );
    }
    let changed_source_basis = if changed_paths.is_empty() {
        Vec::new()
    } else {
        current_repository_source.into_iter().collect()
    };
    let meaningful_completion = candidate.state_change.is_some()
        || !changed_paths.is_empty()
        || !candidate.applied_decisions.is_empty()
        || candidate
            .verification
            .iter()
            .any(|fact| fact.state != volicord_context::VerificationState::NotRun)
        || !candidate.known_limits.is_empty();
    if candidate.kind == CheckpointKind::Completion && !meaningful_completion {
        return reject(
            CheckpointRejection::NoMeaningfulChange,
            "completion has no attributable change, applied Decision, verification, or new limit",
            attribution,
        );
    }
    let boundary_valid = match candidate.kind {
        CheckpointKind::Completion => {
            candidate.work_state == WorkState::Completed && candidate.handoff_to.is_none()
        }
        CheckpointKind::Pause => {
            candidate.work_state == WorkState::Paused && candidate.handoff_to.is_none()
        }
        CheckpointKind::Handoff => candidate.handoff_to.is_some(),
    };
    if !boundary_valid {
        return reject(
            CheckpointRejection::InvalidBoundary,
            "Checkpoint kind, work state, and handoff target are inconsistent",
            attribution,
        );
    }
    let open_questions = canonical
        .active_questions
        .iter()
        .map(|question| QuestionReference {
            question_id: question.id,
            revision: question.revision,
        })
        .collect();
    CheckpointEvaluation::Ready {
        draft: Box::new(CheckpointDraft {
            expected_project_revision: canonical.project.revision,
            kind: candidate.kind,
            goal: candidate.goal,
            work_state: candidate.work_state,
            state_change: candidate.state_change,
            source_basis,
            changed_source_basis,
            changed_paths,
            applied_decisions: candidate.applied_decisions,
            verification: candidate.verification,
            user_review: candidate.user_review,
            user_acceptance: candidate.user_acceptance,
            known_limits: candidate.known_limits,
            non_goals: candidate.non_goals,
            open_questions,
            next_step: candidate.next_step,
            handoff_to: candidate.handoff_to,
        }),
        attribution: attribution.unwrap_or(ChangeAttribution::Attributed {
            pre_existing_paths: Vec::new(),
            changed_paths: Vec::new(),
        }),
    }
}

fn reject(
    reason: CheckpointRejection,
    detail: impl Into<String>,
    attribution: Option<ChangeAttribution>,
) -> CheckpointEvaluation {
    CheckpointEvaluation::Rejected {
        reason,
        detail: detail.into(),
        attribution,
    }
}

/// Persists only a successfully evaluated draft through the canonical Kernel.
pub fn record_checkpoint(
    context: &mut Store,
    operation_id: OperationId,
    project_id: ProjectId,
    evaluation: CheckpointEvaluation,
) -> Result<volicord_context::OperationResult<Checkpoint>, crate::Error> {
    let CheckpointEvaluation::Ready { draft, .. } = evaluation else {
        return Err(crate::Error::new(
            crate::ErrorKind::DomainConflict,
            "rejected Checkpoint evaluation cannot be persisted",
        ));
    };
    context
        .record_checkpoint(operation_id, project_id, *draft)
        .map_err(|error| {
            crate::Error::with_source(
                crate::ErrorKind::CanonicalFailure,
                "canonical Checkpoint operation failed",
                error,
            )
        })
}
