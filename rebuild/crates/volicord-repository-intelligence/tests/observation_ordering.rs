use std::{collections::HashMap, error::Error, fs, path::Path};
use volicord_repository_intelligence::{
    analyze_repository_semantics, AnalysisSnapshot, InventoryRequest, RelationTarget,
    SemanticAnalysisRequest, SourceRange, StructuralAnalysisRequest,
};

mod support;

fn copy_tree(from: &Path, to: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn location(range: &Option<SourceRange>) -> serde_json::Value {
    range.as_ref().map_or(serde_json::Value::Null, |range| {
        serde_json::json!([
            range.locator,
            range.start,
            range.end,
            range.coordinate_convention,
            range.meaning,
            range.adapter,
            range.precision_limit
        ])
    })
}

fn graph_order(analysis: &AnalysisSnapshot) -> serde_json::Value {
    let ranks = analysis
        .structural_facts
        .iter()
        .enumerate()
        .map(|(rank, fact)| (fact.entity.identity.as_str(), rank))
        .collect::<HashMap<_, _>>();
    let target = |target: &RelationTarget| match target {
        RelationTarget::ResolvedEntity(id) => serde_json::json!(ranks.get(id.as_str())),
        RelationTarget::Unresolved(value) => serde_json::json!(value),
    };
    serde_json::json!({
        "facts": analysis.structural_facts.iter().map(|fact| serde_json::json!([
            fact.entity.area, fact.entity.language, fact.entity.kind,
            fact.entity.display_name, fact.entity.qualified_name,
            location(&fact.entity.source_range),
            fact.relations.iter().map(|relation| serde_json::json!([
                relation.kind, location(&relation.supporting_range), target(&relation.target)
            ])).collect::<Vec<_>>()
        ])).collect::<Vec<_>>(),
        "semantics": analysis.semantic_results.iter().map(|result| serde_json::json!([
            ranks.get(result.relation.source_entity.as_str()), result.relation.kind,
            location(&result.relation.supporting_range), target(&result.relation.target),
            result.provenance.adapter, result.provenance.analyzer,
            result.provenance.build_context, result.provenance.resolution_basis
        ])).collect::<Vec<_>>()
    })
}

#[test]
fn fresh_observations_preserve_polyglot_graph_order_and_current_bindings(
) -> Result<(), Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../validation/repository-intelligence/polyglot-structural/fixtures");
    for language in [
        "c",
        "cpp",
        "java",
        "javascript",
        "typescript",
        "python",
        "rust",
    ] {
        copy_tree(&fixtures.join(language), &root.path().join(language))?;
    }
    let mut expected_order = None;
    let mut sources = std::collections::BTreeSet::new();
    let mut identities = std::collections::BTreeSet::new();
    for observer in [0x52, 0x53, 0x54] {
        let canonical = support::repository_grounding(0x51, observer)?;
        let inventory = InventoryRequest::new(
            root.path(),
            &canonical.grounding,
            canonical.source_id,
            1_725_000_000_000_000 + i64::from(observer),
        )?;
        let request = StructuralAnalysisRequest::new(inventory);
        let (_, analysis) = analyze_repository_semantics(SemanticAnalysisRequest::new(request))?;
        assert!(sources.insert(analysis.repository_source.identity()));
        assert!(identities.insert(analysis.identity));
        assert!(!analysis.semantic_results.is_empty());
        assert_eq!(
            analysis
                .inventory
                .languages
                .iter()
                .filter(|language| language.is_structural_gate_language())
                .count(),
            7
        );
        for fact in &analysis.structural_facts {
            assert_eq!(fact.entity.analysis_snapshot, analysis.identity);
            assert_eq!(fact.entity.source, analysis.repository_source);
            for relation in &fact.relations {
                assert_eq!(relation.analysis_snapshot, analysis.identity);
                if let Some(range) = &relation.supporting_range {
                    assert_eq!(range.source, analysis.repository_source);
                }
            }
        }
        for result in &analysis.semantic_results {
            assert_eq!(result.relation.analysis_snapshot, analysis.identity);
            if let Some(range) = &result.relation.supporting_range {
                assert_eq!(range.source, analysis.repository_source);
            }
        }
        let order = graph_order(&analysis);
        if let Some(expected) = &expected_order {
            assert!(
                &order == expected,
                "observation identity shuffled the graph"
            );
        } else {
            expected_order = Some(order);
        }
    }
    Ok(())
}
