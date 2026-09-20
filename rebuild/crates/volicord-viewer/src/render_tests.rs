use super::{
    layout_diagram_topology, select_diagram_topology, MapEntity, MapRelation, MapRelationClass,
};
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
        select_diagram_topology(&components, &relationships, 16, |_| true, true);
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
        select_diagram_topology(&components, &relationships, 16, |_| true, true);
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

#[test]
fn flow_diagram_does_not_fill_capacity_with_disconnected_components() {
    let components = (0..12)
        .map(|index| map_entity(format!("generic-{index:02}")))
        .chain([
            map_entity("current-a".into()),
            map_entity("current-b".into()),
        ])
        .collect::<Vec<_>>();
    let relationships = vec![map_relation(
        "flow:current".into(),
        "current-a".into(),
        "current-b".into(),
    )];

    let (nodes, selected_relationships) =
        select_diagram_topology(&components, &relationships, 8, |_| true, false);
    assert_eq!(
        nodes
            .iter()
            .map(|node| node.identity.as_str())
            .collect::<Vec<_>>(),
        vec!["current-a", "current-b"]
    );
    assert_eq!(selected_relationships.len(), 1);

    let (gap_nodes, gap_relationships) =
        select_diagram_topology(&components, &relationships, 8, |_| false, false);
    assert!(gap_nodes.is_empty());
    assert!(gap_relationships.is_empty());
}

#[test]
fn directed_chain_is_laid_out_in_flow_order_independent_of_input_order() {
    let mut components = vec![
        map_entity("middle".into()),
        map_entity("target".into()),
        map_entity("source".into()),
        map_entity("isolated".into()),
    ];
    let mut relationships = vec![
        map_relation("edge:second".into(), "middle".into(), "target".into()),
        map_relation("edge:first".into(), "source".into(), "middle".into()),
    ];
    let component_refs = components.iter().collect::<Vec<_>>();
    let relationship_refs = relationships.iter().collect::<Vec<_>>();
    let first = layout_diagram_topology(&component_refs, &relationship_refs);
    assert!(first.positions["source"].x < first.positions["middle"].x);
    assert!(first.positions["middle"].x < first.positions["target"].x);
    assert_eq!(first.positions["source"].layer, 0);
    assert_eq!(first.positions["middle"].layer, 1);
    assert_eq!(first.positions["target"].layer, 2);

    components.reverse();
    relationships.reverse();
    let component_refs = components.iter().collect::<Vec<_>>();
    let relationship_refs = relationships.iter().collect::<Vec<_>>();
    assert_eq!(
        first,
        layout_diagram_topology(&component_refs, &relationship_refs)
    );
}

#[test]
fn directed_cycle_shares_a_layer_without_inventing_an_order() {
    let components = vec![
        map_entity("cycle-a".into()),
        map_entity("cycle-b".into()),
        map_entity("after-cycle".into()),
    ];
    let relationships = vec![
        map_relation("cycle:a-b".into(), "cycle-a".into(), "cycle-b".into()),
        map_relation("cycle:b-a".into(), "cycle-b".into(), "cycle-a".into()),
        map_relation("cycle:out".into(), "cycle-b".into(), "after-cycle".into()),
    ];
    let layout = layout_diagram_topology(
        &components.iter().collect::<Vec<_>>(),
        &relationships.iter().collect::<Vec<_>>(),
    );
    assert_eq!(
        layout.positions["cycle-a"].layer,
        layout.positions["cycle-b"].layer
    );
    assert!(layout.positions["cycle-b"].x < layout.positions["after-cycle"].x);
    assert_ne!(layout.positions["cycle-a"].y, layout.positions["cycle-b"].y);
}

fn map_entity(identity: String) -> MapEntity {
    let repository_snapshot = repository_snapshot();
    MapEntity {
        display_name: identity.clone(),
        locator: format!("src/{identity}.rs"),
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
