use crate::{
    AnalysisSnapshot, CapabilityState, FreshnessBasis, FreshnessState, RepositorySnapshot,
};

impl AnalysisSnapshot {
    /// Reassesses a read-side copy against an observation using the same canonical
    /// repository Source. Historical identities and persisted analysis are unchanged.
    pub fn observe_repository_freshness(&mut self, observation: Option<&RepositorySnapshot>) {
        let compatible = observation.filter(|value| {
            value.project == self.project
                && value.repository_source == self.repository_source
                && value.unavailable_areas.is_empty()
        });
        if compatible.is_some_and(|value| value.identity == self.repository_snapshot) {
            return;
        }
        let (state, reason) = if compatible.is_some() {
            (FreshnessState::Stale, "the repository has changed since this analysis; refresh analysis before using current code facts")
        } else {
            (FreshnessState::Unknown, "current repository freshness could not be verified; these are historical analysis results")
        };
        let freshness = FreshnessBasis {
            state,
            repository_snapshot: self.repository_snapshot,
            compared_repository_snapshot: compatible.map(|value| value.identity),
            reason: Some(reason.into()),
        };
        self.freshness = freshness.clone();
        for report in &mut self.capabilities {
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
