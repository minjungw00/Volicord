use super::{select_diagram_topology, MapEntity, MapRelation, MapRelationClass};
use volicord_context::SourceId;
use volicord_repository_intelligence::{
    AnalysisSnapshotId, CodeEntityKind, FreshnessBasis, FreshnessState, Language,
    RepositorySnapshotId, Uncertainty,
};

#[test]
fn diagram_bound_keeps_relationship_endpoints_beyond_the_naive_prefix() {
    let mut components = (0..24)
        .map(|index| map_entity(format!("a{index:02}")))
        .chain([map_entity("z00".into()), map_entity("z01".into())])
        .collect::<Vec<_>>();
    let relationship = map_relation("relation:z".into(), "z00".into(), "z01".into());
    let relationships = vec![relationship];

    let (nodes, selected_relationships) =
        select_diagram_topology(&components, &relationships, 16, |_| true);
    let node_ids = nodes
        .iter()
        .map(|node| node.identity.clone())
        .collect::<Vec<_>>();
    let relationship_ids = selected_relationships
        .iter()
        .map(|relation| relation.identity.clone())
        .collect::<Vec<_>>();
    assert_eq!(nodes.len(), 16);
    assert!(node_ids.contains(&"z00".to_owned()));
    assert!(node_ids.contains(&"z01".to_owned()));
    assert_eq!(selected_relationships.len(), 1);

    components.reverse();
    let (reordered_nodes, reordered_relationships) =
        select_diagram_topology(&components, &relationships, 16, |_| true);
    assert_eq!(
        node_ids,
        reordered_nodes
            .iter()
            .map(|node| node.identity.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        relationship_ids,
        reordered_relationships
            .iter()
            .map(|relation| relation.identity.clone())
            .collect::<Vec<_>>()
    );
}

fn map_entity(identity: String) -> MapEntity {
    let repository_snapshot = repository_snapshot();
    MapEntity {
        display_name: identity.clone(),
        identity,
        kind: CodeEntityKind::Module,
        language: Language::Rust,
        source_id: SourceId::from_bytes([1; 16]),
        source_range: None,
        analysis_snapshot: analysis_snapshot(),
        repository_snapshot,
        freshness: FreshnessBasis {
            state: FreshnessState::Current,
            repository_snapshot,
            compared_repository_snapshot: None,
            reason: None,
        },
        uncertainty: Uncertainty::none(),
        canonical_links: Vec::new(),
    }
}

fn map_relation(identity: String, source: String, target: String) -> MapRelation {
    let repository_snapshot = repository_snapshot();
    MapRelation {
        identity,
        class: MapRelationClass::StructuralFact,
        kind: "Imports".into(),
        source_entity: source,
        target_entity: Some(target),
        unresolved_target: None,
        source_id: SourceId::from_bytes([1; 16]),
        supporting_range: None,
        analysis_snapshot: analysis_snapshot(),
        repository_snapshot,
        freshness: FreshnessBasis {
            state: FreshnessState::Current,
            repository_snapshot,
            compared_repository_snapshot: None,
            reason: None,
        },
        uncertainty: Uncertainty::none(),
        diagnostics: Vec::new(),
    }
}

fn repository_snapshot() -> RepositorySnapshotId {
    RepositorySnapshotId::from_hex(&"11".repeat(32))
        .unwrap_or_else(|error| panic!("valid Repository Snapshot fixture identity: {error}"))
}

fn analysis_snapshot() -> AnalysisSnapshotId {
    AnalysisSnapshotId::from_hex(&"22".repeat(32))
        .unwrap_or_else(|error| panic!("valid Analysis Snapshot fixture identity: {error}"))
}
