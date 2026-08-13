use serde::Deserialize;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use volicord_context::{ProjectId, SourceId};
use volicord_repository_intelligence::{
    analyze_repository, canonical_json, search_local, Capability, CapabilityState, CodeEntityKind,
    CoordinateConvention, FreshnessState, InvalidationCategory, InventoryRequest, Language,
    RelationTarget, SearchResultKind, StructuralAnalysisRequest, StructuralRelationKind,
};

const OBSERVED_AT: i64 = 1_725_000_000_000_000;

#[derive(Deserialize)]
struct Manifest {
    fixtures: Vec<Fixture>,
}

#[derive(Deserialize)]
struct Fixture {
    id: String,
    validation_id: String,
    path: String,
    expected_entities: Vec<String>,
    expected_relations: Vec<String>,
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn fixture(name: &str) -> PathBuf {
    repository_root()
        .join("rebuild/validation/repository-intelligence/polyglot-structural/fixtures")
        .join(name)
}

fn inventory(root: &Path) -> InventoryRequest<'_> {
    InventoryRequest::new(
        root,
        ProjectId::from_bytes([0x51; 16]),
        SourceId::from_bytes([0x52; 16]),
        OBSERVED_AT,
    )
}

#[test]
fn all_seven_adapters_satisfy_maintained_entities_ranges_and_relations(
) -> Result<(), Box<dyn Error>> {
    let manifest: Manifest = serde_json::from_slice(&fs::read(
        repository_root().join("rebuild/validation/shared/fixture-manifest.json"),
    )?)?;
    for fixture in manifest
        .fixtures
        .into_iter()
        .filter(|fixture| fixture.validation_id == "V01" && !fixture.expected_entities.is_empty())
    {
        let root = repository_root().join(&fixture.path);
        let (repository, analysis) =
            analyze_repository(StructuralAnalysisRequest::new(inventory(&root)))?;
        for expected in fixture.expected_entities {
            let parts = expected.split('|').collect::<Vec<_>>();
            assert_eq!(parts.len(), 5, "invalid expected entity: {expected}");
            let language = language(parts[0])?;
            let kind = entity_kind(parts[2])?;
            let line: u64 = parts[4].parse()?;
            let found = analysis.structural_facts.iter().any(|fact| {
                fact.entity.language == language
                    && fact.entity.area.path == parts[1]
                    && fact.entity.kind == kind
                    && fact.entity.display_name.as_deref() == Some(parts[3])
                    && fact.entity.source_range.as_ref().is_some_and(|range| {
                        range.repository_snapshot == repository.identity
                            && range.coordinate_convention
                                == CoordinateConvention::ZeroBasedUtf8Byte
                            && range.start.line + 1 == line
                            && (range.end.line, range.end.column)
                                > (range.start.line, range.start.column)
                    })
            });
            assert!(found, "{} missing entity {expected}", fixture.id);
        }
        for expected in fixture.expected_relations {
            let parts = expected.split('|').collect::<Vec<_>>();
            assert_eq!(parts.len(), 3, "invalid expected relation: {expected}");
            let kind = relation_kind(parts[0])?;
            let found = analysis.structural_facts.iter().any(|fact| {
                fact.entity.qualified_name.as_deref() == Some(parts[1])
                    && fact.relations.iter().any(|relation| {
                        relation.kind == kind && relation_target(&relation.target) == parts[2]
                    })
            });
            assert!(found, "{} missing relation {expected}", fixture.id);
        }
        for language in analysis
            .inventory
            .languages
            .iter()
            .filter(|language| language.is_structural_gate_language())
        {
            let capability = analysis.capabilities.iter().find(|report| {
                report.language.as_ref() == Some(language)
                    && report.capability == Capability::Structural
            });
            assert!(capability.is_some_and(|report| {
                matches!(
                    report.state,
                    CapabilityState::Available | CapabilityState::Partial
                ) && report.coverage.covered_entity_count > 0
            }));
            if fixture.id != "v01-polyglot" {
                let ecosystem = analysis.capabilities.iter().find(|report| {
                    report.language.as_ref() == Some(language)
                        && report.capability == Capability::Ecosystem
                });
                assert!(ecosystem.is_some_and(|report| {
                    report.state == CapabilityState::Partial
                        && !report.coverage.included.is_empty()
                        && report.analyzer.is_none()
                }));
            }
        }
        assert!(analysis.semantic_results.is_empty());
        assert!(analysis.semantic_annotations.is_empty());
        assert!(analysis.agent_interpretations.is_empty());
    }
    Ok(())
}

#[test]
fn same_snapshot_facts_and_serialization_are_deterministic() -> Result<(), Box<dyn Error>> {
    let first_binding = tempfile::tempdir()?;
    let second_binding = tempfile::tempdir()?;
    copy_tree(&fixture("polyglot"), first_binding.path())?;
    copy_tree(&fixture("polyglot"), second_binding.path())?;
    let (_, first) = analyze_repository(StructuralAnalysisRequest::new(inventory(
        first_binding.path(),
    )))?;
    let (_, repeated) = analyze_repository(StructuralAnalysisRequest::new(inventory(
        first_binding.path(),
    )))?;
    let (_, rebound) = analyze_repository(StructuralAnalysisRequest::new(inventory(
        second_binding.path(),
    )))?;
    assert_eq!(first.identity, repeated.identity);
    assert_eq!(first.identity, rebound.identity);
    assert_eq!(first.structural_facts, repeated.structural_facts);
    assert_eq!(first.structural_facts, rebound.structural_facts);
    assert_eq!(canonical_json(&first)?, canonical_json(&repeated)?);
    assert_eq!(canonical_json(&first)?, canonical_json(&rebound)?);
    Ok(())
}

#[test]
fn changed_file_and_declared_dependent_reparse_without_whole_repository(
) -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    copy_tree(&fixture("typescript"), temporary.path())?;
    let (_, before) =
        analyze_repository(StructuralAnalysisRequest::new(inventory(temporary.path())))?;
    fs::write(
        temporary.path().join("src/suffix.ts"),
        "export const suffix = \"!\";\n",
    )?;
    let (repository, refreshed) = analyze_repository(
        StructuralAnalysisRequest::new(inventory(temporary.path())).with_previous(&before),
    )?;
    assert_eq!(refreshed.refresh.parsed_file_count, 2);
    assert_eq!(refreshed.refresh.reused_file_count, 1);
    assert!(refreshed.invalidations.iter().any(|record| {
        record.area.path == "src/suffix.ts" && record.category == InvalidationCategory::FileContent
    }));
    assert!(refreshed.invalidations.iter().any(|record| {
        record.area.path == "src/index.ts"
            && record.category == InvalidationCategory::Dependency
            && record
                .dependency_area
                .as_ref()
                .is_some_and(|area| area.path == "src/suffix.ts")
    }));
    assert!(refreshed.structural_facts.iter().all(|fact| {
        fact.entity.repository_snapshot == repository.identity
            && fact.entity.freshness.state == FreshnessState::Current
            && fact
                .entity
                .source_range
                .as_ref()
                .is_some_and(|range| range.repository_snapshot == repository.identity)
    }));

    let (_, full) =
        analyze_repository(StructuralAnalysisRequest::new(inventory(temporary.path())))?;
    assert_eq!(refreshed.identity, full.identity);
    let refreshed_by_id = refreshed
        .structural_facts
        .iter()
        .map(|fact| (fact.entity.identity.as_str(), fact))
        .collect::<BTreeMap<_, _>>();
    let full_by_id = full
        .structural_facts
        .iter()
        .map(|fact| (fact.entity.identity.as_str(), fact))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        refreshed_by_id.keys().collect::<Vec<_>>(),
        full_by_id.keys().collect::<Vec<_>>()
    );
    for (identity, refreshed_fact) in refreshed_by_id {
        assert_eq!(
            Some(refreshed_fact),
            full_by_id.get(identity).copied(),
            "{identity}"
        );
    }
    Ok(())
}

#[test]
fn manifest_change_uses_explicit_build_context_invalidation() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    copy_tree(&fixture("typescript"), temporary.path())?;
    let (_, before) =
        analyze_repository(StructuralAnalysisRequest::new(inventory(temporary.path())))?;
    let package = temporary.path().join("package.json");
    let mut content = fs::read_to_string(&package)?;
    content.push('\n');
    fs::write(package, content)?;
    let (_, refreshed) = analyze_repository(
        StructuralAnalysisRequest::new(inventory(temporary.path())).with_previous(&before),
    )?;
    assert_eq!(refreshed.refresh.reused_file_count, 0);
    assert_eq!(refreshed.refresh.parsed_file_count, 3);
    assert_eq!(
        refreshed
            .invalidations
            .iter()
            .filter(|record| record.category == InvalidationCategory::BuildContext)
            .count(),
        3
    );
    Ok(())
}

#[test]
fn search_is_source_grounded_and_stale_ranges_are_not_current_navigation(
) -> Result<(), Box<dyn Error>> {
    let root = fixture("rust");
    let (repository, analysis) =
        analyze_repository(StructuralAnalysisRequest::new(inventory(&root)))?;
    let current = search_local(&analysis, "Greeter.greet", repository.identity, 10);
    assert!(current.iter().any(|hit| {
        hit.result_kind == SearchResultKind::Entity
            && hit.capability == Capability::Structural
            && hit.source_range.is_some()
            && hit.coverage.covered_entity_count > 0
            && hit.freshness.state == FreshnessState::Current
            && !hit.diagnostics.is_empty()
            && hit.navigation_is_current
    }));
    let other_root = fixture("python");
    let (other_repository, _) =
        analyze_repository(StructuralAnalysisRequest::new(inventory(&other_root)))?;
    let stale = search_local(&analysis, "Greeter.greet", other_repository.identity, 10);
    assert!(stale.iter().any(|hit| {
        hit.freshness.state == FreshnessState::Stale
            && hit.freshness.compared_repository_snapshot == Some(other_repository.identity)
            && !hit.navigation_is_current
            && hit.source_range.is_some()
    }));
    Ok(())
}

#[test]
fn out_of_set_language_keeps_inventory_fallback_without_structural_facts(
) -> Result<(), Box<dyn Error>> {
    let root = fixture("out_of_set");
    let (_, analysis) = analyze_repository(StructuralAnalysisRequest::new(inventory(&root)))?;
    assert!(analysis.inventory.languages.contains(&Language::Go));
    assert!(analysis.structural_facts.is_empty());
    let structural = analysis.capabilities.iter().find(|report| {
        report.language == Some(Language::Go) && report.capability == Capability::Structural
    });
    assert!(structural.is_some_and(|report| report.state == CapabilityState::Unsupported));
    Ok(())
}

fn language(value: &str) -> Result<Language, Box<dyn Error>> {
    Ok(match value {
        "java" => Language::Java,
        "python" => Language::Python,
        "javascript" => Language::JavaScript,
        "typescript" => Language::TypeScript,
        "c" => Language::C,
        "cpp" => Language::Cpp,
        "rust" => Language::Rust,
        _ => return Err(format!("unknown language: {value}").into()),
    })
}

fn entity_kind(value: &str) -> Result<CodeEntityKind, Box<dyn Error>> {
    Ok(match value {
        "class" => CodeEntityKind::Class,
        "interface" => CodeEntityKind::Interface,
        "trait" => CodeEntityKind::Trait,
        "struct" => CodeEntityKind::Struct,
        "enum" => CodeEntityKind::Enum,
        "type" => CodeEntityKind::Type,
        "function" => CodeEntityKind::Function,
        "method" => CodeEntityKind::Method,
        "field" => CodeEntityKind::Field,
        "test" => CodeEntityKind::Test,
        _ => return Err(format!("unknown entity kind: {value}").into()),
    })
}

fn relation_kind(value: &str) -> Result<StructuralRelationKind, Box<dyn Error>> {
    Ok(match value {
        "imports" => StructuralRelationKind::Imports,
        "includes" => StructuralRelationKind::Includes,
        "exports" => StructuralRelationKind::Exports,
        "inherits" => StructuralRelationKind::Inherits,
        "implements" => StructuralRelationKind::Implements,
        "calls_syntactically" => StructuralRelationKind::CallsSyntactically,
        "tests" => StructuralRelationKind::Tests,
        _ => return Err(format!("unknown relation kind: {value}").into()),
    })
}

fn relation_target(target: &RelationTarget) -> &str {
    match target {
        RelationTarget::ResolvedEntity(identity) => identity,
        RelationTarget::Unresolved(target) => &target.display,
    }
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            fs::create_dir_all(&destination_path)?;
            copy_tree(&source_path, &destination_path)?;
        } else {
            fs::copy(source_path, destination_path)?;
        }
    }
    Ok(())
}
