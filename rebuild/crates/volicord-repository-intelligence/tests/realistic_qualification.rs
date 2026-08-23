use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use volicord_repository_intelligence::{
    analyze_repository, analyze_repository_semantics, canonical_json, AnalysisSnapshot, Capability,
    CapabilityState, CodeEntityKind, CoordinateConvention, InventoryRequest, Language,
    ProvenanceClass, RelationTarget, SemanticAnalysisRequest, SemanticRelationKind,
    StructuralAnalysisRequest,
};

mod support;

const OBSERVED_AT: i64 = 1_725_000_000_000_000;

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn fixtures() -> PathBuf {
    repository_root()
        .join("rebuild/validation/repository-intelligence/realistic-qualification/fixtures")
}

fn inventory(root: &Path) -> Result<InventoryRequest<'_>, Box<dyn Error>> {
    let canonical = support::repository_grounding(0x91, 0x92)?;
    Ok(InventoryRequest::new(
        root,
        &canonical.grounding,
        canonical.source_id,
        OBSERVED_AT,
    )?)
}

#[test]
fn all_seven_realistic_repositories_are_multi_file_partial_and_source_ranged(
) -> Result<(), Box<dyn Error>> {
    let matrix = [
        (
            "java",
            Language::Java,
            "src/main/java/acme/service/GreetingService.java",
            CodeEntityKind::Class,
            "GreetingService",
        ),
        (
            "python",
            Language::Python,
            "src/atlas/service.py",
            CodeEntityKind::Class,
            "Catalog",
        ),
        (
            "javascript",
            Language::JavaScript,
            "src/core/parse.js",
            CodeEntityKind::Class,
            "Parser",
        ),
        (
            "typescript",
            Language::TypeScript,
            "src/service.ts",
            CodeEntityKind::Class,
            "GreetingService",
        ),
        (
            "c",
            Language::C,
            "src/record.c",
            CodeEntityKind::Function,
            "record_normalize",
        ),
        (
            "cpp",
            Language::Cpp,
            "src/model.cpp",
            CodeEntityKind::Function,
            "normalize",
        ),
        (
            "rust",
            Language::Rust,
            "crates/catalog/src/service.rs",
            CodeEntityKind::Struct,
            "Catalog",
        ),
    ];

    for (fixture, language, path, kind, name) in matrix {
        let root = fixtures().join(fixture);
        let (repository, analysis) =
            analyze_repository(StructuralAnalysisRequest::new(inventory(&root)?))?;
        let source_files = analysis
            .inventory
            .entries
            .iter()
            .filter(|entry| entry.language.as_ref() == Some(&language))
            .count();
        assert!(
            source_files >= 4,
            "{fixture} is not a realistic multi-file input"
        );
        assert!(
            analysis.structural_facts.iter().any(|fact| {
                fact.entity.language == language
                    && fact.entity.area.path == path
                    && fact.entity.kind == kind
                    && fact.entity.display_name.as_deref() == Some(name)
            }),
            "{fixture} missing representative entity {path}:{name}"
        );
        assert!(
            analysis.structural_facts.iter().all(|fact| {
                fact.entity.source_range.as_ref().is_some_and(|range| {
                    range.repository_snapshot == repository.identity
                        && range.coordinate_convention == CoordinateConvention::ZeroBasedUtf8Byte
                        && (range.start.line, range.start.column)
                            < (range.end.line, range.end.column)
                })
            }),
            "{fixture} emitted an invalid or fabricated range"
        );
        let report = analysis
            .capabilities
            .iter()
            .find(|report| {
                report.language == Some(language.clone())
                    && report.capability == Capability::Structural
            })
            .ok_or("structural capability report missing")?;
        assert_eq!(report.state, CapabilityState::Partial, "{fixture}");
        assert!(report.coverage.covered_file_count > 0, "{fixture}");
        assert!(report.coverage.covered_entity_count > 0, "{fixture}");
        assert!(!report.diagnostics.is_empty(), "{fixture}");
        assert!(
            analysis.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "structural.parse_error"
                    && diagnostic.affected_area.path.contains("broken")
            }),
            "{fixture} hid its damaged source"
        );
        assert!(analysis.semantic_results.is_empty(), "{fixture}");
    }
    Ok(())
}

#[test]
fn realistic_same_name_declarations_remain_scope_distinct() -> Result<(), Box<dyn Error>> {
    for (fixture, language, name, minimum) in [
        ("java", Language::Java, "GreetingService", 2),
        ("python", Language::Python, "Record", 2),
        ("javascript", Language::JavaScript, "parse", 2),
        ("typescript", Language::TypeScript, "GreetingService", 2),
        ("c", Language::C, "normalize", 2),
        ("cpp", Language::Cpp, "Record", 2),
        ("rust", Language::Rust, "Catalog", 2),
    ] {
        let root = fixtures().join(fixture);
        let (_, analysis) = analyze_repository(StructuralAnalysisRequest::new(inventory(&root)?))?;
        let matches = analysis
            .structural_facts
            .iter()
            .filter(|fact| {
                fact.entity.language == language
                    && fact.entity.display_name.as_deref() == Some(name)
            })
            .collect::<Vec<_>>();
        assert!(matches.len() >= minimum, "{fixture}:{name}");
        assert_eq!(
            matches
                .iter()
                .map(|fact| fact.entity.identity.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            matches.len(),
            "{fixture}:{name} identities were conflated"
        );
        assert!(
            matches
                .iter()
                .map(|fact| fact.entity.area.path.as_str())
                .collect::<BTreeSet<_>>()
                .len()
                >= 2,
            "{fixture}:{name} did not exercise distinct files/scopes"
        );
    }
    Ok(())
}

#[test]
fn incremental_refresh_matches_full_analysis_and_preserves_unaffected_ranges(
) -> Result<(), Box<dyn Error>> {
    let temporary = tempdir()?;
    copy_tree(&fixtures().join("typescript"), temporary.path())?;
    let (_, before) =
        analyze_repository(StructuralAnalysisRequest::new(inventory(temporary.path())?))?;
    let (_, repeated) =
        analyze_repository(StructuralAnalysisRequest::new(inventory(temporary.path())?))?;
    assert_eq!(before.identity, repeated.identity);
    assert_eq!(canonical_json(&before)?, canonical_json(&repeated)?);

    let stable_before = entity_range(&before, "src/other/service.ts", "GreetingService")?;
    let changed_source = temporary.path().join("src/names.ts");
    fs::write(
        &changed_source,
        fs::read_to_string(&changed_source)?.replace("value.trim()", "value.trimStart()"),
    )?;
    let (_, refreshed) = analyze_repository(
        StructuralAnalysisRequest::new(inventory(temporary.path())?).with_previous(&before),
    )?;
    let (_, full) =
        analyze_repository(StructuralAnalysisRequest::new(inventory(temporary.path())?))?;

    assert!(refreshed.refresh.parsed_file_count > 0);
    assert!(refreshed.refresh.reused_file_count > 0);
    assert_eq!(
        structural_projection(&refreshed),
        structural_projection(&full),
        "incremental facts differ from a full analysis of the same source snapshot"
    );
    assert_eq!(
        stable_before,
        entity_range(&refreshed, "src/other/service.ts", "GreetingService")?,
        "unaffected source range moved during incremental refresh"
    );
    Ok(())
}

#[test]
fn curated_semantic_queries_keep_per_ecosystem_precision_and_recall() -> Result<(), Box<dyn Error>>
{
    let cases = [
        SemanticCase {
            fixture: "java",
            language: Language::Java,
            queries: vec![
                query(
                    SemanticRelationKind::Implements,
                    "src/main/java/acme/service/GreetingService.java",
                    "GreetingService",
                    6,
                    "src/main/java/acme/api/GreetingPort.java",
                    "GreetingPort",
                ),
                query(
                    SemanticRelationKind::References,
                    "src/main/java/acme/service/GreetingService.java",
                    "render",
                    8,
                    "src/main/java/acme/util/Names.java",
                    "normalize",
                ),
            ],
        },
        SemanticCase {
            fixture: "typescript",
            language: Language::TypeScript,
            queries: vec![
                query(
                    SemanticRelationKind::Implements,
                    "src/service.ts",
                    "GreetingService",
                    4,
                    "src/api.ts",
                    "GreetingPort",
                ),
                query(
                    SemanticRelationKind::References,
                    "src/service.ts",
                    "render",
                    7,
                    "src/names.ts",
                    "normalize",
                ),
            ],
        },
        SemanticCase {
            fixture: "rust",
            language: Language::Rust,
            queries: vec![query(
                SemanticRelationKind::Implements,
                "crates/catalog/src/service.rs",
                "Catalog",
                11,
                "crates/catalog/src/service.rs",
                "Named",
            )],
        },
    ];

    for case in cases {
        let root = fixtures().join(case.fixture);
        let (_, analysis) = analyze_repository_semantics(SemanticAnalysisRequest::new(
            StructuralAnalysisRequest::new(inventory(&root)?),
        ))?;
        let report = analysis
            .capabilities
            .iter()
            .find(|report| {
                report.language == Some(case.language.clone())
                    && report.capability == Capability::Semantic
            })
            .ok_or("semantic capability report missing")?;
        assert_eq!(report.state, CapabilityState::Partial, "{}", case.fixture);
        assert!(
            report.coverage.covered_relation_count > 0,
            "{}",
            case.fixture
        );
        assert_eq!(report.provenance_class, ProvenanceClass::SemanticResult);

        let mut true_positive = 0_u64;
        let mut false_positive = 0_u64;
        for expected in &case.queries {
            let sources = entities_matching(
                &analysis,
                expected.source_path,
                expected.source_name,
                expected.source_line,
            );
            assert!(!sources.is_empty(), "{} source missing", case.fixture);
            let candidates = analysis.semantic_results.iter().filter(|result| {
                sources.contains(result.relation.source_entity.as_str())
                    && result.relation.kind == expected.kind
            });
            let mut matched = false;
            for candidate in candidates {
                let correct = match &candidate.relation.target {
                    RelationTarget::ResolvedEntity(identity) => analysis
                        .structural_facts
                        .iter()
                        .find(|fact| fact.entity.identity == *identity)
                        .is_some_and(|fact| {
                            fact.entity.area.path == expected.target_path
                                && fact.entity.display_name.as_deref() == Some(expected.target_name)
                        }),
                    RelationTarget::Unresolved(_) => false,
                };
                if correct {
                    matched = true;
                } else if matches!(candidate.relation.target, RelationTarget::ResolvedEntity(_)) {
                    false_positive += 1;
                }
            }
            true_positive += u64::from(matched);
        }
        let false_negative = case.queries.len() as u64 - true_positive;
        let precision = true_positive as f64 / (true_positive + false_positive).max(1) as f64;
        let recall = true_positive as f64 / (true_positive + false_negative).max(1) as f64;
        assert!(precision >= 1.0, "{} precision={precision}", case.fixture);
        assert!(recall >= 1.0, "{} recall={recall}", case.fixture);
        assert!(
            analysis.semantic_results.iter().any(|result| {
                matches!(result.relation.target, RelationTarget::Unresolved(_))
                    && !result.relation.uncertainty.reasons.is_empty()
            }),
            "{} hid unsupported or unresolved semantic targets",
            case.fixture
        );
    }
    Ok(())
}

#[test]
fn polyglot_boundary_is_config_and_schema_grounded_without_completeness_generalization(
) -> Result<(), Box<dyn Error>> {
    let root = fixtures().join("polyglot");
    let (_, analysis) = analyze_repository(StructuralAnalysisRequest::new(inventory(&root)?))?;
    let system: serde_json::Value = serde_json::from_slice(&fs::read(root.join("system.json"))?)?;
    let components = system["components"]
        .as_object()
        .ok_or("component manifest missing")?;
    for path in components.values().filter_map(serde_json::Value::as_str) {
        assert!(analysis
            .inventory
            .entries
            .iter()
            .any(|entry| entry.area.path == path));
        assert!(analysis
            .structural_facts
            .iter()
            .any(|fact| fact.entity.area.path == path));
    }

    let schema_path = system["schema"].as_str().ok_or("schema path missing")?;
    let schema = fs::read_to_string(root.join(schema_path))?;
    assert!(schema.contains("greeting.request.v1"));
    for path in [
        "java/src/main/java/relay/Gateway.java",
        "python/src/relay_worker/worker.py",
        "typescript/src/client.ts",
    ] {
        assert!(fs::read_to_string(root.join(path))?.contains("greeting.request.v1"));
    }
    for path in [
        "java/src/main/java/relay/Gateway.java",
        "typescript/src/client.ts",
    ] {
        assert!(fs::read_to_string(root.join(path))?.contains("/v1/greet"));
    }

    let per_language = analysis
        .capabilities
        .iter()
        .filter(|report| report.capability == Capability::Structural)
        .filter_map(|report| report.language.clone())
        .collect::<BTreeSet<_>>();
    assert!(per_language.contains(&Language::Java));
    assert!(per_language.contains(&Language::Python));
    assert!(per_language.contains(&Language::TypeScript));
    assert!(
        !analysis.capabilities.iter().any(|report| {
            report.capability == Capability::Structural && report.language.is_none()
        }),
        "polyglot structural completeness was generalized repository-wide"
    );
    Ok(())
}

#[test]
fn external_repositories_when_provided_are_bounded_and_honest() -> Result<(), Box<dyn Error>> {
    let Some(state) = std::env::var_os("VOLICORD_EXTERNAL_CORPUS_ROOT") else {
        eprintln!("external corpus: environment_blocked (ignored checkout root not provided)");
        return Ok(());
    };
    let manifest: serde_json::Value = serde_json::from_slice(&fs::read(
        repository_root().join(
            "rebuild/validation/repository-intelligence/realistic-qualification/external-repositories.json",
        ),
    )?)?;
    let repositories = manifest["repositories"]
        .as_array()
        .ok_or("external repositories missing")?;
    for repository in repositories {
        let identifier = repository["id"].as_str().ok_or("external id missing")?;
        let root = Path::new(&state).join(identifier).join("checkout");
        let identity: serde_json::Value = serde_json::from_slice(&fs::read(
            root.parent()
                .ok_or("external checkout has no state parent")?
                .join("input-identity.json"),
        )?)?;
        assert_eq!(identity["origin"], repository["origin"], "{identifier}");
        assert_eq!(identity["revision"], repository["revision"], "{identifier}");
        assert_eq!(identity["license"], repository["license"], "{identifier}");

        let (_, analysis) = analyze_repository(StructuralAnalysisRequest::new(inventory(&root)?))?;
        assert!(analysis.inventory.entries.len() >= 10, "{identifier}");
        for language in repository["expected_languages"]
            .as_array()
            .ok_or("expected languages missing")?
            .iter()
            .map(|value| value.as_str().ok_or("invalid expected language"))
        {
            let language = qualification_language(language?)?;
            assert!(
                analysis.inventory.languages.contains(&language),
                "{identifier}"
            );
            let report = analysis.capabilities.iter().find(|report| {
                report.language == Some(language.clone())
                    && report.capability == Capability::Structural
            });
            assert!(
                report.is_some_and(|report| {
                    matches!(
                        report.state,
                        CapabilityState::Available | CapabilityState::Partial
                    ) && report.coverage.covered_file_count > 0
                        && report.coverage.covered_entity_count > 0
                }),
                "{identifier}:{language:?}"
            );
        }
        assert!(
            !analysis.capabilities.iter().any(|report| {
                report.capability == Capability::Structural && report.language.is_none()
            }),
            "{identifier} generalized structural completeness repository-wide"
        );
    }
    Ok(())
}

#[derive(Clone)]
struct SemanticQuery {
    kind: SemanticRelationKind,
    source_path: &'static str,
    source_name: &'static str,
    source_line: u64,
    target_path: &'static str,
    target_name: &'static str,
}

struct SemanticCase {
    fixture: &'static str,
    language: Language,
    queries: Vec<SemanticQuery>,
}

fn query(
    kind: SemanticRelationKind,
    source_path: &'static str,
    source_name: &'static str,
    source_line: u64,
    target_path: &'static str,
    target_name: &'static str,
) -> SemanticQuery {
    SemanticQuery {
        kind,
        source_path,
        source_name,
        source_line,
        target_path,
        target_name,
    }
}

fn qualification_language(value: &str) -> Result<Language, Box<dyn Error>> {
    Ok(match value {
        "java" => Language::Java,
        "python" => Language::Python,
        "javascript" => Language::JavaScript,
        "typescript" => Language::TypeScript,
        "c" => Language::C,
        "cpp" => Language::Cpp,
        "rust" => Language::Rust,
        _ => return Err(format!("unsupported qualification language: {value}").into()),
    })
}

fn entities_matching<'a>(
    analysis: &'a AnalysisSnapshot,
    path: &str,
    name: &str,
    line: u64,
) -> BTreeSet<&'a str> {
    analysis
        .structural_facts
        .iter()
        .filter(|fact| {
            fact.entity.area.path == path
                && fact.entity.display_name.as_deref() == Some(name)
                && fact
                    .entity
                    .source_range
                    .as_ref()
                    .is_some_and(|range| range.start.line + 1 == line)
        })
        .map(|fact| fact.entity.identity.as_str())
        .collect()
}

fn structural_projection(analysis: &AnalysisSnapshot) -> BTreeSet<String> {
    let entities = analysis
        .structural_facts
        .iter()
        .map(|fact| {
            let range = fact.entity.source_range.as_ref().map(|range| {
                (
                    range.start.line,
                    range.start.column,
                    range.end.line,
                    range.end.column,
                )
            });
            format!(
                "entity|{:?}|{}|{:?}|{}|{range:?}",
                fact.entity.language,
                fact.entity.area.path,
                fact.entity.kind,
                fact.entity.qualified_name.as_deref().unwrap_or_default(),
            )
        })
        .chain(analysis.structural_facts.iter().flat_map(|fact| {
            fact.relations.iter().map(|relation| {
                let target = match &relation.target {
                    RelationTarget::ResolvedEntity(identity) => analysis
                        .structural_facts
                        .iter()
                        .find(|candidate| candidate.entity.identity == *identity)
                        .map(|candidate| {
                            format!(
                                "{}:{}",
                                candidate.entity.area.path,
                                candidate
                                    .entity
                                    .qualified_name
                                    .as_deref()
                                    .unwrap_or_default()
                            )
                        })
                        .unwrap_or_else(|| "missing-resolved-target".to_owned()),
                    RelationTarget::Unresolved(target) => format!("unresolved:{}", target.display),
                };
                format!(
                    "relation|{}|{}|{:?}|{target}",
                    fact.entity.area.path,
                    fact.entity.qualified_name.as_deref().unwrap_or_default(),
                    relation.kind,
                )
            })
        }))
        .collect::<BTreeSet<_>>();
    entities
}

fn entity_range(
    analysis: &AnalysisSnapshot,
    path: &str,
    name: &str,
) -> Result<(u64, u64, u64, u64), Box<dyn Error>> {
    let range = analysis
        .structural_facts
        .iter()
        .find(|fact| {
            fact.entity.area.path == path && fact.entity.display_name.as_deref() == Some(name)
        })
        .and_then(|fact| fact.entity.source_range.as_ref())
        .ok_or("entity range missing")?;
    Ok((
        range.start.line,
        range.start.column,
        range.end.line,
        range.end.column,
    ))
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
