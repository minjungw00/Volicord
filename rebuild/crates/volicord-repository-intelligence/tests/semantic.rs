use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use volicord_context::{
    ApplicabilityScope, Availability, CanonicalReadOptions, CheckpointId,
    ContextItemCorrectionDraft, ContextItemDraft, ContextItemRole, CorrectionKind, DecisionId,
    DeterministicIdGenerator, FixedClock, OperationId, Principal, PrincipalKind, ProjectId,
    SourceDraft, SourceId, SourcePayload, StatementProvenanceRole, Store, TimestampMicros,
};
use volicord_repository_intelligence::{
    analyze_repository_semantics, canonical_json, grounded_explanation_basis, search_local,
    AgentInterpretation, CanonicalCheckpointRef, CanonicalContextItemRef, CanonicalDecisionRef,
    CanonicalLinkSelector, CanonicalReference, CanonicalSourceRef, Capability, CapabilityState,
    CoordinateConvention, FreshnessState, GroundingStatementClass, InventoryRequest, Language,
    ProvenanceClass, RelationTarget, SearchResultKind, SemanticAnalysisRequest,
    SemanticRelationKind, StructuralAnalysisRequest, Uncertainty, UncertaintyLevel,
    ANALYSIS_SNAPSHOT_FORMAT_VERSION,
};

const OBSERVED_AT: i64 = 1_725_000_000_000_000;

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
        ProjectId::from_bytes([0x71; 16]),
        SourceId::from_bytes([0x72; 16]),
        OBSERVED_AT,
    )
}

#[test]
fn three_production_ecosystems_publish_normalized_semantic_relations() -> Result<(), Box<dyn Error>>
{
    for (fixture_name, language) in [
        ("java", Language::Java),
        ("typescript", Language::TypeScript),
        ("rust", Language::Rust),
    ] {
        let root = fixture(fixture_name);
        let (repository, analysis) = analyze_repository_semantics(SemanticAnalysisRequest::new(
            StructuralAnalysisRequest::new(inventory(&root)),
        ))?;
        assert_eq!(analysis.format_version, ANALYSIS_SNAPSHOT_FORMAT_VERSION);
        let report = analysis.capabilities.iter().find(|report| {
            report.language == Some(language.clone()) && report.capability == Capability::Semantic
        });
        assert!(report.is_some_and(|report| {
            matches!(
                report.state,
                CapabilityState::Available | CapabilityState::Partial
            ) && report.coverage.covered_relation_count > 0
                && report.analyzer.as_ref().is_some_and(|analyzer| {
                    analyzer.name == "volicord-source-semantic-index" && analyzer.version == "1"
                })
                && report.provenance_class == ProvenanceClass::SemanticResult
        }));
        let kinds = analysis
            .semantic_results
            .iter()
            .filter(|result| semantic_source_language(&analysis, result) == Some(&language))
            .map(|result| result.relation.kind.clone())
            .collect::<BTreeSet<_>>();
        for required in [
            SemanticRelationKind::Defines,
            SemanticRelationKind::References,
            SemanticRelationKind::ResolvesTo,
            SemanticRelationKind::TypeOf,
            SemanticRelationKind::Implements,
            SemanticRelationKind::Overrides,
        ] {
            assert!(
                kinds.contains(&required),
                "{fixture_name} missing {required:?}"
            );
        }
        assert!(analysis
            .semantic_results
            .iter()
            .filter(|result| semantic_source_language(&analysis, result) == Some(&language))
            .all(|result| {
                result.relation.repository_snapshot == repository.identity
                    && result.relation.analysis_snapshot == analysis.identity
                    && result.provenance.analysis.analysis_snapshot == analysis.identity
                    && result.provenance.analysis.class == ProvenanceClass::SemanticResult
                    && result
                        .relation
                        .supporting_range
                        .as_ref()
                        .is_some_and(|range| {
                            range.repository_snapshot == repository.identity
                                && range.coordinate_convention
                                    == CoordinateConvention::ZeroBasedUtf8Byte
                                && range.adapter.name.ends_with("-source-semantic")
                        })
            }));

        let overrides = analysis
            .semantic_results
            .iter()
            .filter(|result| result.relation.kind == SemanticRelationKind::Overrides)
            .collect::<Vec<_>>();
        assert!(
            !overrides.is_empty(),
            "{fixture_name} has no override-style relation"
        );
        assert!(overrides.iter().any(|result| {
            let Some(source) = entity(&analysis, &result.relation.source_entity) else {
                return false;
            };
            let RelationTarget::ResolvedEntity(target) = &result.relation.target else {
                return false;
            };
            let Some(target) = entity(&analysis, target) else {
                return false;
            };
            source.display_name == target.display_name && source.identity != target.identity
        }));
        assert!(analysis
            .structural_facts
            .iter()
            .all(|fact| { fact.provenance.analysis.class == ProvenanceClass::StructuralFact }));
    }
    Ok(())
}

#[test]
fn unavailable_or_failed_adapter_cannot_publish_semantic_facts() -> Result<(), Box<dyn Error>> {
    let root = fixture("rust");
    let (_, analysis) = analyze_repository_semantics(
        SemanticAnalysisRequest::new(StructuralAnalysisRequest::new(inventory(&root)))
            .with_unavailable_language(Language::Rust, "adapter disabled for this analysis"),
    )?;
    assert!(analysis.semantic_results.is_empty());
    assert!(!analysis.structural_facts.is_empty());
    assert!(analysis.capabilities.iter().any(|report| {
        report.language == Some(Language::Rust)
            && report.capability == Capability::Semantic
            && report.state == CapabilityState::Unavailable
            && report.coverage.covered_relation_count == 0
            && report.reason.as_deref() == Some("adapter disabled for this analysis")
    }));
    Ok(())
}

#[test]
fn broken_dependency_is_partial_without_erasing_usable_remainder() -> Result<(), Box<dyn Error>> {
    for (fixture_name, language, locator, marker) in [
        (
            "java",
            Language::Java,
            "src/main/java/example/Greeter.java",
            "import missing.Dependency;\n",
        ),
        (
            "typescript",
            Language::TypeScript,
            "src/index.ts",
            "import { absent } from './missing.js';\n",
        ),
        (
            "rust",
            Language::Rust,
            "crates/greeter/src/lib.rs",
            "use missing::Dependency;\n",
        ),
    ] {
        let temporary = tempdir()?;
        copy_tree(&fixture(fixture_name), temporary.path())?;
        let source = temporary.path().join(locator);
        fs::write(&source, format!("{marker}{}", fs::read_to_string(&source)?))?;
        let (repository, analysis) = analyze_repository_semantics(SemanticAnalysisRequest::new(
            StructuralAnalysisRequest::new(inventory(temporary.path())),
        ))?;
        let semantic = analysis.capabilities.iter().find(|report| {
            report.language == Some(language.clone()) && report.capability == Capability::Semantic
        });
        assert!(
            semantic.is_some_and(|report| {
                report.state == CapabilityState::Partial
                    && report.coverage.covered_relation_count > 0
                    && report.usable_remainder.is_some()
                    && !report.diagnostics.is_empty()
            }),
            "{fixture_name}"
        );
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.code == "semantic.unresolved_dependency" }),
            "{fixture_name}"
        );
        assert!(!analysis.inventory.entries.is_empty(), "{fixture_name}");
        assert!(!analysis.structural_facts.is_empty(), "{fixture_name}");
        assert!(!analysis.semantic_results.is_empty(), "{fixture_name}");
        let hits = search_local(&analysis, "missing", repository.identity, 20);
        assert!(
            hits.iter().any(|hit| {
                hit.result_kind == SearchResultKind::SemanticRelation
                    && hit.capability == Capability::Semantic
                    && hit.provenance_class == ProvenanceClass::SemanticResult
            }),
            "{fixture_name}"
        );
    }
    Ok(())
}

#[test]
fn overloads_are_distinct_and_calls_resolve_by_arity() -> Result<(), Box<dyn Error>> {
    let temporary = tempdir()?;
    fs::create_dir_all(temporary.path().join("src/main/java/example"))?;
    fs::write(
        temporary.path().join("pom.xml"),
        "<project><modelVersion>4.0.0</modelVersion></project>\n",
    )?;
    fs::write(
        temporary.path().join("src/main/java/example/Greeter.java"),
        r#"package example;
interface Named { String greet(String person); }
class Greeter implements Named {
    public String greet(String person) { return person; }
    public String greet(int repeat, String person) { return person.repeat(repeat); }
    String call() { return greet("Ada"); }
}
"#,
    )?;
    let (_, analysis) = analyze_repository_semantics(SemanticAnalysisRequest::new(
        StructuralAnalysisRequest::new(inventory(temporary.path())),
    ))?;
    let overloads = analysis
        .structural_facts
        .iter()
        .filter(|fact| fact.entity.qualified_name.as_deref() == Some("example.Greeter.greet"))
        .collect::<Vec<_>>();
    assert_eq!(overloads.len(), 2);
    assert_ne!(overloads[0].entity.identity, overloads[1].entity.identity);
    let call = analysis
        .structural_facts
        .iter()
        .find(|fact| fact.entity.qualified_name.as_deref() == Some("example.Greeter.call"))
        .ok_or("call method missing")?;
    let target = analysis
        .semantic_results
        .iter()
        .find(|result| {
            result.relation.source_entity == call.entity.identity
                && result.relation.kind == SemanticRelationKind::References
        })
        .ok_or("resolved call reference missing")?;
    let RelationTarget::ResolvedEntity(target_identity) = &target.relation.target else {
        return Err("one-argument overload remained unresolved".into());
    };
    let target = entity(&analysis, target_identity).ok_or("overload target missing")?;
    let target_line = target
        .source_range
        .as_ref()
        .ok_or("overload target range missing")?
        .start
        .line;
    assert_eq!(target_line, 3, "call resolved to the wrong overload");
    let overrides = analysis
        .semantic_results
        .iter()
        .filter(|result| result.relation.kind == SemanticRelationKind::Overrides)
        .collect::<Vec<_>>();
    assert_eq!(
        overrides.len(),
        1,
        "two-argument overload was conflated with interface method"
    );
    Ok(())
}

#[test]
fn restart_cache_rebuild_and_refresh_are_deterministic() -> Result<(), Box<dyn Error>> {
    let root = fixture("typescript");
    let (_, first) = analyze_repository_semantics(SemanticAnalysisRequest::new(
        StructuralAnalysisRequest::new(inventory(&root)),
    ))?;
    let (_, restarted) = analyze_repository_semantics(SemanticAnalysisRequest::new(
        StructuralAnalysisRequest::new(inventory(&root)),
    ))?;
    let (_, refreshed) = analyze_repository_semantics(SemanticAnalysisRequest::new(
        StructuralAnalysisRequest::new(inventory(&root)).with_previous(&first),
    ))?;
    assert_eq!(first.identity, restarted.identity);
    assert_eq!(first.identity, refreshed.identity);
    assert_eq!(canonical_json(&first)?, canonical_json(&restarted)?);
    assert_eq!(first.structural_facts, refreshed.structural_facts);
    assert_eq!(first.semantic_results, refreshed.semantic_results);
    assert_eq!(first.semantic_bases, refreshed.semantic_bases);
    assert_eq!(first.semantic_refresh.analyzed_file_count, 3);
    assert_eq!(refreshed.semantic_refresh.analyzed_file_count, 3);
    assert_eq!(refreshed.refresh.reused_file_count, 3);
    Ok(())
}

#[test]
fn explanation_search_and_canonical_links_preserve_authority_and_freshness(
) -> Result<(), Box<dyn Error>> {
    let runtime = tempdir()?;
    let mut store = Store::open_with(
        runtime.path().join("context.sqlite3"),
        DeterministicIdGenerator::new([[0x71; 16], [0x72; 16], [0x73; 16], [0x74; 16]]),
        FixedClock::new(TimestampMicros::from_unix_micros(OBSERVED_AT)),
    )?;
    let project = store
        .create_project(OperationId::from_bytes([0x81; 16]), "Semantic linkage")?
        .value;
    let source = store
        .record_source(
            OperationId::from_bytes([0x82; 16]),
            project.id,
            SourceDraft {
                expected_project_revision: 1,
                payload: SourcePayload::File {
                    locator: "src/index.ts".to_owned(),
                    snapshot: "fixture-snapshot".to_owned(),
                },
                actor: Principal {
                    kind: PrincipalKind::Repository,
                    identity: "repository-observer".to_owned(),
                },
                observer: None,
                availability: Availability::Available,
            },
        )?
        .value;
    let authorization = store
        .record_source(
            OperationId::from_bytes([0x83; 16]),
            project.id,
            SourceDraft {
                expected_project_revision: 1,
                payload: SourcePayload::CurrentHostUserTurn {
                    host: "codex".to_owned(),
                    session: "semantic-linkage".to_owned(),
                    turn: "correct-context".to_owned(),
                },
                actor: Principal {
                    kind: PrincipalKind::User,
                    identity: "owner".to_owned(),
                },
                observer: None,
                availability: Availability::Available,
            },
        )?
        .value;
    let context = store
        .record_context_item(
            OperationId::from_bytes([0x84; 16]),
            project.id,
            ContextItemDraft {
                expected_project_revision: 1,
                role: ContextItemRole::Fact,
                statement: "Greeter semantics are source grounded.".to_owned(),
                provenance_role: StatementProvenanceRole::Observed,
                author: Principal {
                    kind: PrincipalKind::Agent,
                    identity: "codex".to_owned(),
                },
                source_basis: vec![source.id],
                applicability: ApplicabilityScope::default(),
            },
        )?
        .value;
    let corrected = store
        .correct_context_item(
            OperationId::from_bytes([0x85; 16]),
            project.id,
            context.id,
            ContextItemCorrectionDraft {
                expected_revision: 1,
                corrected_statement: "Greeter semantics are source-grounded.".to_owned(),
                kind: CorrectionKind::Typography,
                user_authorization_source_id: authorization.id,
            },
        )?
        .value;
    assert_eq!(corrected.revision, 2);
    let before = store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;

    let root = fixture("typescript");
    let request_inventory = InventoryRequest::new(&root, project.id, source.id, OBSERVED_AT);
    let (repository, mut analysis) = analyze_repository_semantics(
        SemanticAnalysisRequest::new(StructuralAnalysisRequest::new(request_inventory))
            .with_canonical_link(
                CanonicalLinkSelector::new(Language::TypeScript, "src/index.ts", "Greeter.greet"),
                CanonicalReference::Decision(CanonicalDecisionRef(DecisionId::from_bytes(
                    [0x73; 16],
                ))),
            )
            .with_canonical_link(
                CanonicalLinkSelector::new(Language::TypeScript, "src/index.ts", "Greeter.greet"),
                CanonicalReference::ContextItem(CanonicalContextItemRef(context.id)),
            )
            .with_canonical_link(
                CanonicalLinkSelector::new(Language::TypeScript, "src/index.ts", "Greeter.greet"),
                CanonicalReference::Checkpoint(CanonicalCheckpointRef(CheckpointId::from_bytes(
                    [0x75; 16],
                ))),
            ),
    )?;
    let after = store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
    assert_eq!(before, after, "analysis refresh mutated canonical context");
    let derived = runtime.path().join("analysis-derived.json");
    fs::write(&derived, canonical_json(&analysis)?)?;
    fs::remove_file(&derived)?;
    let (_, rebuilt) = analyze_repository_semantics(SemanticAnalysisRequest::new(
        StructuralAnalysisRequest::new(InventoryRequest::new(
            &root,
            project.id,
            source.id,
            OBSERVED_AT,
        )),
    ))?;
    fs::write(&derived, canonical_json(&rebuilt)?)?;
    assert_eq!(
        before,
        store.read_canonical_basis(project.id, CanonicalReadOptions::default())?,
        "deleting and rebuilding Derived State changed canonical context"
    );
    analysis.agent_interpretations.push(AgentInterpretation {
        identity: "agent-interpretation-1".to_owned(),
        analysis_snapshot: analysis.identity,
        agent: "codex".to_owned(),
        host: "codex".to_owned(),
        session: "semantic-test".to_owned(),
        source_basis: vec![CanonicalSourceRef(source.id)],
        analysis_basis: vec![analysis.identity.to_string()],
        text: "Greeter may be affected; this remains an interpretation.".to_owned(),
        generated_at_unix_micros: OBSERVED_AT,
        known_gaps: vec!["runtime dispatch was not observed".to_owned()],
        uncertainty: Uncertainty {
            level: UncertaintyLevel::Medium,
            reasons: vec!["impact is an interpretation, not a correctness claim".to_owned()],
        },
        provenance_class: ProvenanceClass::AgentInterpretation,
    });
    let greeter = analysis
        .structural_facts
        .iter()
        .find(|fact| fact.entity.qualified_name.as_deref() == Some("Greeter.greet"))
        .ok_or("Greeter.greet missing")?;
    assert_eq!(greeter.entity.canonical_links.len(), 4);

    let basis = grounded_explanation_basis(&analysis, repository.identity);
    assert!(!basis.background_source_transmitted);
    assert!(basis
        .evidence
        .iter()
        .any(|evidence| { evidence.statement_class == GroundingStatementClass::StructuralFact }));
    assert!(basis.evidence.iter().any(|evidence| {
        evidence.statement_class == GroundingStatementClass::SemanticResult
            && evidence.provenance_class == ProvenanceClass::SemanticResult
            && evidence.source_range.is_some()
    }));
    assert!(basis.evidence.iter().any(|evidence| {
        evidence.statement_class == GroundingStatementClass::AgentInterpretation
            && evidence.provenance_class == ProvenanceClass::AgentInterpretation
            && evidence.uncertainty.level == UncertaintyLevel::Medium
    }));
    assert!(!basis.gaps.is_empty());
    assert!(basis.evidence.iter().any(|evidence| {
        evidence
            .canonical_links
            .iter()
            .any(|link| matches!(link, CanonicalReference::Decision(_)))
    }));

    let semantic_hits = search_local(&analysis, "overrides", repository.identity, 20);
    assert!(semantic_hits.iter().any(|hit| {
        hit.result_kind == SearchResultKind::SemanticRelation
            && hit.provenance_class == ProvenanceClass::SemanticResult
            && hit.navigation_is_current
    }));
    let other = volicord_repository_intelligence::RepositorySnapshotId::from_hex(&"ab".repeat(32))?;
    let stale = grounded_explanation_basis(&analysis, other);
    assert_eq!(stale.freshness.state, FreshnessState::Stale);
    assert!(stale
        .evidence
        .iter()
        .all(|evidence| { evidence.freshness.state == FreshnessState::Stale }));
    Ok(())
}

fn semantic_source_language<'a>(
    analysis: &'a volicord_repository_intelligence::AnalysisSnapshot,
    result: &volicord_repository_intelligence::SemanticAnalysisResult,
) -> Option<&'a Language> {
    entity(analysis, &result.relation.source_entity).map(|entity| &entity.language)
}

fn entity<'a>(
    analysis: &'a volicord_repository_intelligence::AnalysisSnapshot,
    identity: &str,
) -> Option<&'a volicord_repository_intelligence::CodeEntity> {
    analysis
        .structural_facts
        .iter()
        .find(|fact| fact.entity.identity == identity)
        .map(|fact| &fact.entity)
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
