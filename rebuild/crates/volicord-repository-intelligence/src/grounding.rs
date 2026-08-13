use crate::{
    AnalysisSnapshot, Capability, CapabilityState, FreshnessBasis, FreshnessState,
    GroundedExplanationBasis, GroundingEvidence, GroundingGap, GroundingStatementClass,
    ProvenanceClass, RepositorySnapshotId, Uncertainty, UncertaintyLevel,
};

pub fn grounded_explanation_basis(
    analysis: &AnalysisSnapshot,
    current_repository_snapshot: RepositorySnapshotId,
) -> GroundedExplanationBasis {
    let current = analysis.repository_snapshot == current_repository_snapshot;
    let freshness = if current {
        analysis.freshness.clone()
    } else {
        FreshnessBasis {
            state: FreshnessState::Stale,
            repository_snapshot: analysis.repository_snapshot,
            compared_repository_snapshot: Some(current_repository_snapshot),
            reason: Some(
                "the explanation basis belongs to a different Repository Snapshot".to_owned(),
            ),
        }
    };
    let mut evidence = Vec::new();
    for entry in &analysis.inventory.entries {
        evidence.push(GroundingEvidence {
            identity: format!("inventory:{}", entry.area.path),
            statement_class: GroundingStatementClass::RepositoryObservation,
            source: analysis.repository_source,
            source_range: None,
            capability: Capability::Inventory,
            provenance_class: ProvenanceClass::RepositoryObservation,
            freshness: freshness.clone(),
            diagnostics: entry.diagnostic_ids.clone(),
            uncertainty: Uncertainty::none(),
            canonical_links: vec![crate::CanonicalReference::Source(
                analysis.repository_source,
            )],
        });
    }
    for fact in &analysis.structural_facts {
        evidence.push(GroundingEvidence {
            identity: fact.entity.identity.clone(),
            statement_class: GroundingStatementClass::StructuralFact,
            source: fact.entity.source,
            source_range: fact.entity.source_range.clone(),
            capability: Capability::Structural,
            provenance_class: ProvenanceClass::StructuralFact,
            freshness: if current {
                fact.entity.freshness.clone()
            } else {
                freshness.clone()
            },
            diagnostics: fact.entity.diagnostics.clone(),
            uncertainty: fact.entity.uncertainty.clone(),
            canonical_links: fact.entity.canonical_links.clone(),
        });
    }
    for result in &analysis.semantic_results {
        let source = analysis
            .structural_facts
            .iter()
            .find(|fact| fact.entity.identity == result.relation.source_entity)
            .map_or(analysis.repository_source, |fact| fact.entity.source);
        let canonical_links = analysis
            .structural_facts
            .iter()
            .find(|fact| fact.entity.identity == result.relation.source_entity)
            .map_or_else(Vec::new, |fact| fact.entity.canonical_links.clone());
        evidence.push(GroundingEvidence {
            identity: result.relation.identity.clone(),
            statement_class: GroundingStatementClass::SemanticResult,
            source,
            source_range: result.relation.supporting_range.clone(),
            capability: Capability::Semantic,
            provenance_class: ProvenanceClass::SemanticResult,
            freshness: if current {
                result.relation.freshness.clone()
            } else {
                freshness.clone()
            },
            diagnostics: result.relation.diagnostics.clone(),
            uncertainty: result.relation.uncertainty.clone(),
            canonical_links,
        });
    }
    for interpretation in &analysis.agent_interpretations {
        evidence.push(GroundingEvidence {
            identity: interpretation.identity.clone(),
            statement_class: GroundingStatementClass::AgentInterpretation,
            source: interpretation
                .source_basis
                .first()
                .copied()
                .unwrap_or(analysis.repository_source),
            source_range: None,
            capability: Capability::AgentAssisted,
            provenance_class: ProvenanceClass::AgentInterpretation,
            freshness: freshness.clone(),
            diagnostics: interpretation.known_gaps.clone(),
            uncertainty: interpretation.uncertainty.clone(),
            canonical_links: interpretation
                .source_basis
                .iter()
                .copied()
                .map(crate::CanonicalReference::Source)
                .collect(),
        });
    }
    evidence.sort_by(|left, right| left.identity.cmp(&right.identity));
    let mut gaps = analysis
        .capabilities
        .iter()
        .filter(|report| report.state != CapabilityState::Available)
        .map(|report| GroundingGap {
            capability: report.capability,
            language: report.language.clone(),
            state: report.state,
            reason: report
                .reason
                .clone()
                .unwrap_or_else(|| "capability coverage is incomplete".to_owned()),
            affected_areas: report
                .coverage
                .unsupported
                .iter()
                .chain(&report.coverage.unavailable)
                .chain(&report.coverage.failed)
                .chain(&report.coverage.stale)
                .cloned()
                .collect(),
        })
        .collect::<Vec<_>>();
    if !current {
        gaps.push(GroundingGap {
            capability: Capability::Inventory,
            language: None,
            state: CapabilityState::Stale,
            reason: freshness.reason.clone().unwrap_or_default(),
            affected_areas: Vec::new(),
        });
    }
    if evidence.is_empty() {
        evidence.push(GroundingEvidence {
            identity: "analysis:no-evidence".to_owned(),
            statement_class: GroundingStatementClass::RepositoryObservation,
            source: analysis.repository_source,
            source_range: None,
            capability: Capability::Inventory,
            provenance_class: ProvenanceClass::RepositoryObservation,
            freshness: freshness.clone(),
            diagnostics: vec!["the analysis snapshot contains no usable evidence".to_owned()],
            uncertainty: Uncertainty {
                level: UncertaintyLevel::Unknown,
                reasons: vec!["no evidence was produced".to_owned()],
            },
            canonical_links: Vec::new(),
        });
    }
    GroundedExplanationBasis {
        analysis_snapshot: analysis.identity,
        repository_snapshot: analysis.repository_snapshot,
        evidence,
        gaps,
        coverage: analysis.capabilities.clone(),
        freshness,
        background_source_transmitted: false,
    }
}
