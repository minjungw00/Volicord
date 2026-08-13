use crate::model::{
    AnalysisSnapshot, AreaId, AreaKind, Capability, Coverage, FreshnessBasis, FreshnessState,
    InventoryClassification, ProvenanceClass, RelationTarget, SearchHit, SearchResultKind,
    StructuralRelationKind,
};
use crate::RepositorySnapshotId;

pub fn search_local(
    analysis: &AnalysisSnapshot,
    query: &str,
    current_repository_snapshot: RepositorySnapshotId,
    limit: usize,
) -> Vec<SearchHit> {
    let query = query.trim().to_lowercase();
    if query.is_empty() || limit == 0 {
        return Vec::new();
    }
    let mut scored = Vec::new();
    let current = analysis.repository_snapshot == current_repository_snapshot;
    let freshness = if current {
        analysis.freshness.clone()
    } else {
        FreshnessBasis {
            state: FreshnessState::Stale,
            repository_snapshot: analysis.repository_snapshot,
            compared_repository_snapshot: Some(current_repository_snapshot),
            reason: Some("the search index is bound to a different Repository Snapshot".to_owned()),
        }
    };
    let inventory_coverage = capability_coverage(analysis, None, Capability::Inventory);
    let inventory_diagnostics = capability_diagnostics(analysis, None, Capability::Inventory);

    for entry in &analysis.inventory.entries {
        if !entry
            .classifications
            .contains(&InventoryClassification::Included)
        {
            continue;
        }
        if let Some(score) = match_score(&entry.area.path, &query) {
            scored.push((
                score,
                format!("inventory:{}", entry.area.path),
                SearchHit {
                    analysis_snapshot: analysis.identity,
                    repository_snapshot: analysis.repository_snapshot,
                    source: analysis.repository_source,
                    source_range: None,
                    result_kind: SearchResultKind::Inventory,
                    matched_text: entry.area.path.clone(),
                    capability: Capability::Inventory,
                    coverage: inventory_coverage.clone(),
                    freshness: freshness.clone(),
                    diagnostics: merged_diagnostics(&entry.diagnostic_ids, &inventory_diagnostics),
                    provenance_class: ProvenanceClass::RepositoryObservation,
                    navigation_is_current: current,
                },
            ));
        }
    }

    for fact in &analysis.structural_facts {
        let entity = &fact.entity;
        let searchable = format!(
            "{} {} {}",
            entity.display_name.as_deref().unwrap_or_default(),
            entity.qualified_name.as_deref().unwrap_or_default(),
            entity.area.path
        );
        if let Some(score) = match_score(&searchable, &query) {
            scored.push((
                score + 100,
                format!("entity:{}", entity.identity),
                SearchHit {
                    analysis_snapshot: analysis.identity,
                    repository_snapshot: analysis.repository_snapshot,
                    source: entity.source,
                    source_range: entity.source_range.clone(),
                    result_kind: SearchResultKind::Entity,
                    matched_text: entity
                        .qualified_name
                        .clone()
                        .or_else(|| entity.display_name.clone())
                        .unwrap_or_else(|| entity.area.path.clone()),
                    capability: Capability::Structural,
                    coverage: capability_coverage(
                        analysis,
                        Some(&entity.language),
                        Capability::Structural,
                    ),
                    freshness: if current {
                        entity.freshness.clone()
                    } else {
                        freshness.clone()
                    },
                    diagnostics: merged_diagnostics(
                        &entity.diagnostics,
                        &capability_diagnostics(
                            analysis,
                            Some(&entity.language),
                            Capability::Structural,
                        ),
                    ),
                    provenance_class: ProvenanceClass::StructuralFact,
                    navigation_is_current: current
                        && entity.source_range.as_ref().is_some_and(|range| {
                            range.repository_snapshot == current_repository_snapshot
                        }),
                },
            ));
        }
        for relation in &fact.relations {
            let target = relation_target_text(&relation.target);
            let searchable = format!(
                "{} {} {}",
                relation_kind_text(&relation.kind),
                entity.qualified_name.as_deref().unwrap_or_default(),
                target
            );
            if let Some(score) = match_score(&searchable, &query) {
                scored.push((
                    score + 50,
                    format!("relation:{}", relation.identity),
                    SearchHit {
                        analysis_snapshot: analysis.identity,
                        repository_snapshot: analysis.repository_snapshot,
                        source: entity.source,
                        source_range: relation.supporting_range.clone(),
                        result_kind: SearchResultKind::Relation,
                        matched_text: format!(
                            "{} {} -> {}",
                            entity.qualified_name.as_deref().unwrap_or_default(),
                            relation_kind_text(&relation.kind),
                            target
                        ),
                        capability: Capability::Structural,
                        coverage: capability_coverage(
                            analysis,
                            Some(&entity.language),
                            Capability::Structural,
                        ),
                        freshness: if current {
                            relation.freshness.clone()
                        } else {
                            freshness.clone()
                        },
                        diagnostics: merged_diagnostics(
                            &merged_diagnostics(&entity.diagnostics, &relation.diagnostics),
                            &capability_diagnostics(
                                analysis,
                                Some(&entity.language),
                                Capability::Structural,
                            ),
                        ),
                        provenance_class: ProvenanceClass::StructuralFact,
                        navigation_is_current: current
                            && relation.supporting_range.as_ref().is_some_and(|range| {
                                range.repository_snapshot == current_repository_snapshot
                            }),
                    },
                ));
            }
        }
    }

    scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    scored
        .into_iter()
        .take(limit)
        .map(|(_, _, hit)| hit)
        .collect()
}

fn capability_coverage(
    analysis: &AnalysisSnapshot,
    language: Option<&crate::Language>,
    capability: Capability,
) -> Coverage {
    analysis
        .capabilities
        .iter()
        .find(|report| report.language.as_ref() == language && report.capability == capability)
        .map(|report| report.coverage.clone())
        .unwrap_or_else(|| Coverage {
            unavailable: vec![AreaId {
                kind: AreaKind::Repository,
                path: ".".to_owned(),
            }],
            ..Coverage::default()
        })
}

fn capability_diagnostics(
    analysis: &AnalysisSnapshot,
    language: Option<&crate::Language>,
    capability: Capability,
) -> Vec<String> {
    analysis
        .capabilities
        .iter()
        .find(|report| report.language.as_ref() == language && report.capability == capability)
        .map(|report| report.diagnostics.clone())
        .unwrap_or_default()
}

fn merged_diagnostics(primary: &[String], capability: &[String]) -> Vec<String> {
    let mut diagnostics = primary
        .iter()
        .chain(capability)
        .cloned()
        .collect::<Vec<_>>();
    diagnostics.sort();
    diagnostics.dedup();
    diagnostics
}

fn match_score(value: &str, query: &str) -> Option<u64> {
    let value = value.to_lowercase();
    if value == query {
        Some(30)
    } else if value
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .any(|token| token == query)
    {
        Some(20)
    } else if value.contains(query) {
        Some(10)
    } else {
        None
    }
}

fn relation_target_text(target: &RelationTarget) -> &str {
    match target {
        RelationTarget::ResolvedEntity(identity) => identity,
        RelationTarget::Unresolved(target) => &target.display,
    }
}

fn relation_kind_text(kind: &StructuralRelationKind) -> &'static str {
    match kind {
        StructuralRelationKind::Contains => "contains",
        StructuralRelationKind::Declares => "declares",
        StructuralRelationKind::Imports => "imports",
        StructuralRelationKind::Includes => "includes",
        StructuralRelationKind::Exports => "exports",
        StructuralRelationKind::Inherits => "inherits",
        StructuralRelationKind::Implements => "implements",
        StructuralRelationKind::CallsSyntactically => "calls_syntactically",
        StructuralRelationKind::Tests => "tests",
        StructuralRelationKind::Configures => "configures",
        StructuralRelationKind::LanguageSpecific(_) => "language_specific",
    }
}
