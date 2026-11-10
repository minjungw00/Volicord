//! Exercise Search/grounding on the actual stored graph, including its read cache.
use crate::{analysis_io, LocalOperations, RuntimeLayout};
use std::{error::Error, fs};
use volicord_repository_intelligence::{
    grounded_explanation_basis, search_local, CanonicalGrounding, FreshnessState,
    GroundingStatementClass, RelationTarget, SearchResultKind, SemanticRelationKind,
    UncertaintyLevel,
};

#[test]
fn stored_java_receiver_meaning_survives_restart_and_cache_reads() -> Result<(), Box<dyn Error>> {
    let home = tempfile::tempdir()?;
    let repository = home.path().join("repository");
    fs::create_dir(&repository)?;
    fs::write(
        repository.join("Outer.java"),
        include_str!("../../../validation/repository-intelligence/semantic-binding/Outer.java"),
    )?;
    let layout = RuntimeLayout::new(home.path().join("runtime"))?;
    let operations = LocalOperations::new(layout.clone());
    let project = operations
        .initialize_project("Receiver storage control", Some(&repository))?
        .project
        .id;
    let produced = operations
        .analyze(project, vec![])?
        .value
        .ok_or("analysis missing")?;
    let manifest = fs::read(&produced.stored_at)?;
    drop(operations);
    let restarted = LocalOperations::new(layout);
    let canonical = restarted.canonical_basis(project)?;
    let grounding = CanonicalGrounding::from_read_basis(&canonical)?;
    let analysis = analysis_io::read_analysis_durable(&produced.stored_at)?;
    assert_eq!(analysis, produced.analysis);
    let basis = grounded_explanation_basis(&analysis, analysis.repository_snapshot, &grounding)?;
    let hits = search_local(
        &analysis,
        "references",
        analysis.repository_snapshot,
        100,
        &grounding,
    )?;
    // Independent Java expectations: match exact target declaration bytes/range,
    // including distinct same-spelled ping methods. Lambda `this` is lexical;
    // a named class nested inside an anonymous class owns its own `this`.
    let text = fs::read_to_string(repository.join("Outer.java"))?;
    for (caller, spelling, declaration) in [
        ("local", "this.ping", "void ping() { outerHits++; }"),
        ("lexical", "ping", "void ping() { outerHits++; }"),
        ("lambda", "this.ping", "void ping() { outerHits++; }"),
        (
            "argumentLambda",
            "this.ping",
            "void ping() { outerHits++; }",
        ),
        ("qualified", "Outer.staticPing", "static void staticPing()"),
        (
            "qualifiedInsideAnonymous",
            "Outer.staticPing",
            "static void staticPing()",
        ),
        ("named", "this.ping", "void ping() { nestedHits++; }"),
        ("namedLambda", "this.ping", "void ping() { nestedHits++; }"),
        (
            "namedInsideAnonymous",
            "this.ping",
            "void ping() { nestedHits += 1; }",
        ),
    ] {
        let source = analysis
            .structural_facts
            .iter()
            .find(|f| f.entity.display_name.as_deref() == Some(caller))
            .ok_or("positive caller")?;
        let relations: Vec<_> = analysis
            .semantic_results
            .iter()
            .filter(|r| {
                r.relation.source_entity == source.entity.identity
                    && matches!(
                        r.relation.kind,
                        SemanticRelationKind::References | SemanticRelationKind::ResolvesTo
                    )
                    && r.relation.supporting_range.as_ref().is_some_and(|range| {
                        text.lines()
                            .nth(range.start.line as usize)
                            .and_then(|line| line.get(range.start.column as usize..))
                            .is_some_and(|tail| tail.starts_with(spelling))
                    })
            })
            .collect();
        assert_eq!(relations.len(), 2, "{caller}:{spelling}");
        for result in relations {
            let RelationTarget::ResolvedEntity(id) = &result.relation.target else {
                return Err(format!("{caller}: justified receiver lost").into());
            };
            let target = analysis
                .structural_facts
                .iter()
                .find(|f| f.entity.identity == *id)
                .ok_or("target")?;
            let range = target
                .entity
                .source_range
                .as_ref()
                .ok_or("declaration range")?;
            assert!(
                text.lines()
                    .nth(range.start.line as usize)
                    .is_some_and(|line| line.trim_start().starts_with(declaration)),
                "{caller}: wrong declaration {:?}",
                target.entity
            );
            assert_eq!(result.relation.uncertainty.level, UncertaintyLevel::None);
        }
    }
    for caller in ["anonymous", "anonymousLambda", "deepAnonymous"] {
        let source = analysis
            .structural_facts
            .iter()
            .find(|f| f.entity.display_name.as_deref() == Some(caller))
            .ok_or("caller")?;
        let reference = analysis
            .semantic_results
            .iter()
            .find(|r| {
                r.relation.source_entity == source.entity.identity
                    && r.relation.kind == SemanticRelationKind::References
            })
            .ok_or("reference")?;
        assert!(matches!(
            reference.relation.target,
            RelationTarget::Unresolved(_)
        ));
        assert_eq!(reference.relation.uncertainty.level, UncertaintyLevel::High);
        assert!(!analysis
            .semantic_results
            .iter()
            .any(|r| r.relation.source_entity == source.entity.identity
                && r.relation.kind == SemanticRelationKind::ResolvesTo));
        let hit = hits
            .iter()
            .find(|h| {
                h.result_kind == SearchResultKind::SemanticRelation
                    && h.source_range == reference.relation.supporting_range
            })
            .ok_or("Search lost stored occurrence")?;
        assert_eq!(
            hit.relation_target.as_ref(),
            Some(&reference.relation.target)
        );
        assert_eq!(
            hit.semantic_provenance.as_ref(),
            Some(&reference.provenance)
        );
        assert_eq!(hit.uncertainty, reference.relation.uncertainty);
        assert_eq!(hit.freshness.state, FreshnessState::Current);
        let evidence = basis
            .evidence
            .iter()
            .find(|e| {
                e.identity == reference.relation.identity
                    && e.statement_class == GroundingStatementClass::SemanticResult
            })
            .ok_or("grounding lost stored occurrence")?;
        assert_eq!(evidence.source_range, hit.source_range);
        assert_eq!(evidence.relation_target, hit.relation_target);
        assert_eq!(evidence.semantic_provenance, hit.semantic_provenance);
        assert_eq!(evidence.uncertainty, hit.uncertainty);
    }
    // The ordinary Projection reader builds the rebuildable cache from the same
    // immutable manifest. Neither the cold nor the warm path may alter meaning.
    let projection = restarted.project_projection(project)?;
    let warm = restarted.project_projection(project)?;
    assert_eq!(projection.repository_map, warm.repository_map);
    assert_eq!(analysis_io::read_analysis(&produced.stored_at)?, analysis);
    assert_eq!(fs::read(&produced.stored_at)?, manifest);
    assert_eq!(restarted.canonical_basis(project)?, canonical);
    Ok(())
}
