//! Applicability is supplied by dependency owners, never parsed from UI text.
use crate::{ExplanationSourceUse, ExplanationState, RetainedExplanation};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplanationFailureReason {
    Changed,
    Expired,
    Deleted,
    Withheld,
    ComparisonUnavailable,
    Unavailable,
    Missing,
    Corrupt,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryComparisonState {
    VerifiedAtRead,
    Changed,
    Unavailable,
    #[default]
    NotChecked,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExplainedSnapshot {
    pub analysis_snapshot_id: String,
    pub repository_snapshot_id: String,
    pub repository_source_id: String,
    pub use_kind: ExplanationSourceUse,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct ExplanationApplicability {
    pub explained_snapshots: Vec<ExplainedSnapshot>,
    pub generated_at_unix_micros: Option<i64>,
    pub checked_at_unix_micros: Option<i64>,
    pub repository_comparison: RepositoryComparisonState,
    pub limitation: Option<ExplanationFailureReason>,
}

impl ExplanationApplicability {
    pub fn from_retained(e: &RetainedExplanation) -> Self {
        let mut explained_snapshots = Vec::new();
        for c in e.realization.citations() {
            let snapshot = ExplainedSnapshot {
                analysis_snapshot_id: c.analysis_snapshot_id.clone(),
                repository_snapshot_id: c.repository_snapshot_id.clone(),
                repository_source_id: c.repository_source_id.clone(),
                use_kind: c.use_kind,
            };
            if !explained_snapshots.contains(&snapshot) {
                explained_snapshots.push(snapshot);
            }
        }
        Self {
            explained_snapshots,
            generated_at_unix_micros: Some(e.generated_at_unix_micros),
            ..Self::default()
        }
    }
    pub fn unavailable(state: ExplanationState) -> Self {
        use ExplanationFailureReason as F;
        let limitation = match state {
            ExplanationState::Current => None,
            ExplanationState::Stale => Some(F::Changed),
            ExplanationState::Missing => Some(F::Missing),
            ExplanationState::Deleted => Some(F::Deleted),
            ExplanationState::Withheld => Some(F::Withheld),
            ExplanationState::Unsupported => Some(F::Unsupported),
            ExplanationState::Corrupt => Some(F::Corrupt),
            ExplanationState::Unavailable => Some(F::Unavailable),
        };
        Self {
            limitation,
            ..Self::default()
        }
    }
}
