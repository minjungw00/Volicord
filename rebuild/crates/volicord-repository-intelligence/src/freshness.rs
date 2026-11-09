use crate::{
    AnalysisSnapshot, CapabilityState, FreshnessBasis, FreshnessState, RepositorySnapshot,
};

impl AnalysisSnapshot {
    /// Read-side invalidation only. The original producer, target, snapshot and
    /// retained publication basis are never rewritten or relabelled.
    pub fn semantic_contract_is_current(&self) -> bool {
        !superseded(&self.capabilities)
            && self
                .semantic_results
                .iter()
                .all(|r| crate::semantic::current_analyzer(&r.provenance.analyzer))
    }
    pub fn observe_producer_contract(&mut self) {
        if self.semantic_contract_is_current() {
            return;
        }
        let freshness = producer_freshness(&self.freshness);
        invalidate_reports(&mut self.capabilities, &freshness);
        self.freshness = freshness.clone();
        for result in &mut self.semantic_results {
            result.relation.freshness = freshness.clone();
        }
    }

    /// Reassesses a read-side copy against an observation using the same canonical
    /// repository Source. Historical identities and persisted analysis are unchanged.
    pub fn observe_repository_freshness(&mut self, observation: Option<&RepositorySnapshot>) {
        let Some(freshness) = observe_metadata(
            self.project,
            &self.repository_source,
            self.repository_snapshot,
            &mut self.freshness,
            &mut self.capabilities,
            observation,
        ) else {
            return;
        };
        let reason = freshness.reason.as_deref().unwrap_or_default();
        for fact in &mut self.structural_facts {
            fact.entity.freshness = freshness.clone();
            for relation in &mut fact.relations {
                relation.freshness = freshness.clone();
            }
        }
        for result in &mut self.semantic_results {
            result.relation.freshness = freshness.clone();
        }
        for interpretation in &mut self.agent_interpretations {
            interpretation.known_gaps.push(reason.into());
            interpretation.known_gaps.sort();
            interpretation.known_gaps.dedup();
        }
        for annotation in &mut self.semantic_annotations {
            annotation.uncertainty.level = crate::UncertaintyLevel::Unknown;
            annotation.uncertainty.reasons.push(reason.into());
            annotation.uncertainty.reasons.sort();
            annotation.uncertainty.reasons.dedup();
        }
    }
}

impl crate::AnalysisMetadata {
    pub fn observe_repository_freshness(&mut self, observation: Option<&RepositorySnapshot>) {
        observe_metadata(
            self.project,
            &self.repository_source,
            self.repository_snapshot,
            &mut self.freshness,
            &mut self.capabilities,
            observation,
        );
    }
}

fn observe_metadata(
    project: crate::CanonicalProjectRef,
    source: &crate::CanonicalSourceRef,
    repository_snapshot: crate::RepositorySnapshotId,
    read_freshness: &mut FreshnessBasis,
    capabilities: &mut [crate::CapabilityReport],
    observation: Option<&RepositorySnapshot>,
) -> Option<FreshnessBasis> {
    let compatible = observation.filter(|value| {
        value.project == project
            && value.repository_source == *source
            && value.unavailable_areas.is_empty()
    });
    let producer_changed = superseded(capabilities);
    if compatible.is_some_and(|value| value.identity == repository_snapshot) {
        if !producer_changed {
            return None;
        }
        let mut freshness = producer_freshness(read_freshness);
        freshness.compared_repository_snapshot = compatible.map(|value| value.identity);
        invalidate_reports(capabilities, &freshness);
        *read_freshness = freshness.clone();
        return Some(freshness);
    }
    let (state, reason) = if compatible.is_some() {
        (FreshnessState::Stale, "the repository has changed since this analysis; refresh analysis before using current code facts")
    } else {
        (FreshnessState::Unknown, "current repository freshness could not be verified; these are historical analysis results")
    };
    let mut freshness = FreshnessBasis {
        state,
        repository_snapshot,
        compared_repository_snapshot: compatible.map(|value| value.identity),
        reason: Some(reason.into()),
    };
    if producer_changed {
        freshness = producer_freshness(&freshness);
    }
    *read_freshness = freshness.clone();
    for report in capabilities {
        report.freshness = freshness.clone();
        if matches!(
            report.state,
            CapabilityState::Available | CapabilityState::Partial
        ) {
            report.state = if state == FreshnessState::Stale {
                CapabilityState::Stale
            } else {
                CapabilityState::Unavailable
            };
            report.reason = Some(match report.reason.take() {
                Some(previous) => format!("{reason}; original capability: {previous}"),
                None => reason.into(),
            });
            let affected = if state == FreshnessState::Stale {
                &mut report.coverage.stale
            } else {
                &mut report.coverage.unavailable
            };
            affected.extend(report.coverage.included.iter().cloned());
            affected.sort();
            affected.dedup();
        }
    }
    Some(freshness)
}

fn superseded(reports: &[crate::CapabilityReport]) -> bool {
    reports.iter().filter_map(|r| r.analyzer.as_ref()).any(|a| {
        a.name == "volicord-source-semantic-index" && !crate::semantic::current_analyzer(a)
    })
}
fn producer_freshness(original: &FreshnessBasis) -> FreshnessBasis {
    let mut freshness = original.clone();
    if freshness.state == FreshnessState::Current {
        freshness.state = FreshnessState::Stale;
    }
    let reason = "semantic analyzer binding contract changed; reanalyze before using current semantic targets";
    if !freshness
        .reason
        .as_deref()
        .is_some_and(|r| r.contains(reason))
    {
        freshness.reason = Some(match freshness.reason {
            Some(previous) => format!("{previous}; {reason}"),
            None => reason.into(),
        });
    }
    freshness
}
fn invalidate_reports(reports: &mut [crate::CapabilityReport], freshness: &FreshnessBasis) {
    for report in reports.iter_mut().filter(|r| {
        matches!(
            r.capability,
            crate::Capability::Semantic | crate::Capability::Ecosystem
        )
    }) {
        report.freshness = freshness.clone();
        if matches!(
            report.state,
            CapabilityState::Available | CapabilityState::Partial
        ) {
            report.state = if freshness.state == FreshnessState::Unknown {
                CapabilityState::Unavailable
            } else {
                CapabilityState::Stale
            };
        }
        report.reason = freshness.reason.clone();
        let areas = if freshness.state == FreshnessState::Unknown {
            &mut report.coverage.unavailable
        } else {
            &mut report.coverage.stale
        };
        areas.extend(report.coverage.included.clone());
        areas.sort();
        areas.dedup();
    }
}

impl crate::SemanticAnalysisResult {
    pub fn read_freshness(&self) -> FreshnessBasis {
        if crate::semantic::current_analyzer(&self.provenance.analyzer) {
            self.relation.freshness.clone()
        } else {
            producer_freshness(&self.relation.freshness)
        }
    }
}
