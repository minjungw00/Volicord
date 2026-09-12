use crate::{
    AnalysisSnapshot, Capability, CapabilityState, RepositorySnapshot,
    RepositoryWorktreeObservation,
};
use sha2::{Digest, Sha256};

impl RepositorySnapshot {
    /// Bounded equality evidence for the observed repository and analysis boundary.
    /// Fresh Source/snapshot identities and observation times are deliberately excluded.
    /// Missing inventory or Git evidence cannot establish equivalence.
    pub fn observation_equivalence_basis(&self, analysis: &AnalysisSnapshot) -> Option<String> {
        if self.identity != analysis.repository_snapshot
            || self.project != analysis.project
            || self.repository_source != analysis.repository_source
            || !self.unavailable_areas.is_empty()
            || !analysis
                .capabilities
                .iter()
                .any(|report| report.capability == Capability::Inventory)
            || analysis.capabilities.iter().any(|report| {
                report.capability == Capability::Inventory
                    && report.state != CapabilityState::Available
            })
            || analysis
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.capability == Capability::Inventory)
            || matches!(
                (&analysis.repository_worktree, &self.observation_basis.git),
                (RepositoryWorktreeObservation::Git { .. }, None)
                    | (RepositoryWorktreeObservation::NonGit, Some(_))
            )
        {
            return None;
        }
        // These typed projections retain capability, area, coverage and producer boundaries,
        // without their Source-bound identities, freshness references or timestamps.
        let capabilities = analysis
            .capabilities
            .iter()
            .map(|report| {
                (
                    &report.language,
                    &report.area,
                    report.capability,
                    report.state,
                    &report.coverage,
                    &report.adapter,
                    &report.analyzer,
                    report.provenance_class,
                )
            })
            .collect::<Vec<_>>();
        let basis = serde_json::to_vec(&(
            "volicord.repository_observation_equivalence.v1",
            self.format_version,
            &analysis.format_kind,
            analysis.format_version,
            self.project,
            &self.source_boundary,
            &self.observation_basis,
            &self.included_areas,
            &self.excluded_areas,
            &analysis.repository_worktree,
            capabilities,
        ))
        .ok()?;
        Some(format!("sha256:{:x}", Sha256::digest(basis)))
    }
}
