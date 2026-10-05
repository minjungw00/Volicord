//! Read-side analysis state. No analyzer, filesystem, provider or write authority.
use serde::{Deserialize, Serialize};
use volicord_repository_intelligence::{
    AnalysisSnapshotId, Capability, CapabilityReport, CapabilityState, FreshnessBasis,
    FreshnessState, InventoryEntry, Language, RepositorySnapshotId,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryAnalysisState {
    Absent,
    Current,
    Partial,
    Stale,
    Failed,
    FreshnessUnknown,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisAttemptReading {
    pub operation_id: String,
    pub completed_at_unix_micros: i64,
    pub failed: bool,
    pub diagnostic: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AnalysisCoverageReading {
    pub capability: Capability,
    pub language: Option<Language>,
    pub area: String,
    pub state: CapabilityState,
    pub files: u64,
    pub entities: u64,
    pub relations: u64,
    pub reason: Option<String>,
    pub usable_remainder: Option<String>,
    pub consequence: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RepositoryAnalysisReading {
    pub state: RepositoryAnalysisState,
    pub analysis_snapshot: Option<AnalysisSnapshotId>,
    pub repository_snapshot: Option<RepositorySnapshotId>,
    pub generated_at_unix_micros: Option<i64>,
    pub freshness: Option<FreshnessBasis>,
    pub coverage: Vec<AnalysisCoverageReading>,
    pub omitted_coverage_count: usize,
    pub latest_attempt: Option<AnalysisAttemptReading>,
    pub latest_attempt_error: Option<String>,
    pub retained_prior_result: bool,
    pub diagnostic: Option<String>,
    /// Existing local operation. This is guidance, never a Viewer mutation URL.
    pub refresh_command: String,
}

impl RepositoryAnalysisReading {
    pub(crate) fn absent(unavailable: bool) -> Self {
        Self {
            state: if unavailable {
                RepositoryAnalysisState::Unavailable
            } else {
                RepositoryAnalysisState::Absent
            },
            analysis_snapshot: None,
            repository_snapshot: None,
            generated_at_unix_micros: None,
            freshness: None,
            coverage: Vec::new(),
            omitted_coverage_count: 0,
            latest_attempt: None,
            latest_attempt_error: None,
            retained_prior_result: false,
            diagnostic: None,
            refresh_command: if unavailable {
                "volicord doctor repair"
            } else {
                "volicord analyze"
            }
            .into(),
        }
    }

    pub(crate) fn stored(
        analysis: AnalysisSnapshotId,
        repository: RepositorySnapshotId,
        generated_at: i64,
        freshness: &FreshnessBasis,
        capabilities: &[CapabilityReport],
        entries: &[InventoryEntry],
    ) -> Self {
        let mut result = Self::absent(false);
        result.analysis_snapshot = Some(analysis);
        result.repository_snapshot = Some(repository);
        result.generated_at_unix_micros = Some(generated_at);
        result.freshness = Some(freshness.clone());
        result.coverage = capabilities
            .iter()
            .filter(|r| {
                r.language.as_ref().is_none_or(|language| {
                    entries.iter().any(|e| {
                        e.language.as_ref() == Some(language)
                            && area_contains(&r.area.path, &e.area.path)
                    })
                })
            })
            .map(|r| AnalysisCoverageReading {
                capability: r.capability,
                language: r.language.clone(),
                area: r.area.path.clone(),
                state: r.state,
                files: r.coverage.covered_file_count,
                entities: r.coverage.covered_entity_count,
                relations: r.coverage.covered_relation_count,
                reason: r.reason.clone(),
                usable_remainder: r.usable_remainder.clone(),
                consequence: r.user_visible_consequence.clone(),
            })
            .collect();
        // Determine state before presentation bounds; freshness and availability
        // remain independent of capability coverage and latest-attempt outcome.
        result.state = match freshness.state {
            FreshnessState::Unknown => RepositoryAnalysisState::FreshnessUnknown,
            FreshnessState::Stale => RepositoryAnalysisState::Stale,
            FreshnessState::Current => {
                if result
                    .coverage
                    .iter()
                    .any(|r| r.state == CapabilityState::Failed)
                {
                    RepositoryAnalysisState::Failed
                } else if result
                    .coverage
                    .iter()
                    .any(|r| r.state != CapabilityState::Available)
                {
                    RepositoryAnalysisState::Partial
                } else {
                    RepositoryAnalysisState::Current
                }
            }
        };
        result.omitted_coverage_count = result.coverage.len().saturating_sub(32);
        result.coverage.truncate(32);
        result
    }

    pub fn observe_attempt(&mut self, attempt: AnalysisAttemptReading) {
        if attempt.failed
            && self
                .generated_at_unix_micros
                .is_none_or(|at| attempt.completed_at_unix_micros >= at)
        {
            self.state = RepositoryAnalysisState::Failed;
            self.retained_prior_result = self.analysis_snapshot.is_some();
            self.diagnostic = attempt.diagnostic.clone();
        }
        self.latest_attempt = Some(attempt);
    }
}

fn area_contains(scope: &str, path: &str) -> bool {
    scope.is_empty()
        || scope == "."
        || scope == path
        || path
            .strip_prefix(scope)
            .is_some_and(|suffix| suffix.starts_with('/'))
}
