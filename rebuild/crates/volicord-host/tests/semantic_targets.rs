//! Independent source-language expectations through the ordinary local producer,
//! persisted analysis, public Search/grounding, Projection and MCP reader.
//! These authored fixtures do not represent naturalistic Work or human acceptance.
use serde_json::json;
use std::{error::Error, fs};
use volicord_context::{
    ApplicabilityScope, ContextItemDraft, ContextItemRole, OperationId, Principal, PrincipalKind,
    StatementProvenanceRole, Store,
};
use volicord_host::HostAdapter;
use volicord_operations::{LocalOperations, RuntimeLayout};
use volicord_projections::MapRelationClass;
use volicord_repository_intelligence::{
    grounded_explanation_basis, search_local, CanonicalGrounding, FreshnessState,
    GroundingStatementClass, RelationTarget, SearchResultKind, SemanticRelationKind,
    UncertaintyLevel,
};

#[test]
fn ordinary_analysis_preserves_binding_meaning_through_restart_and_public_readers(
) -> Result<(), Box<dyn Error>> {
    for (filename, text, expected_name, negative_caller) in [
        ("receiver.ts", include_str!("../../../validation/repository-intelligence/semantic-binding/receiver.ts"), "log", "demo"),
        ("Owner.java", "class Owner {\nstatic void println(String value) {}\nstatic void demo() { System.out.println(\"host\"); }\nstatic void local() { println(\"local\"); }\n}\n", "println", "demo"),
        ("lib.rs", "fn len() {}\nfn demo(value: &str) { value.len(); }\nfn local() { len(); }\n", "len", "demo"),
        ("objects.ts", "function real(value: number) {}\nfunction orphan() { const object = { run() {} }; run(); }\nfunction local() { real(1); }\n", "real", "orphan"),
    ] {
        let home = tempfile::tempdir()?;
        let repository = home.path().join("repository");
        fs::create_dir(&repository)?;
        fs::write(repository.join(filename), text)?;
        let layout = RuntimeLayout::new(home.path().join("runtime"))?;
        let operations = LocalOperations::new(layout.clone());
        let project = operations.initialize_project("Binding reader control", Some(&repository))?.project.id;
        // Explicit authored Work scope seeds the ordinary architecture reader.
        let source = operations.record_user_source(project, "test-fixture".into(), "semantic-binding-control".into(), format!("Inspect {filename}"))?;
        let source_id = operations.canonical_basis(project)?.sources.iter().find(|s| s.source.id.to_string() == source.identity).ok_or("fixture Source")?.source.id;
        let mut store = Store::open(layout.canonical_store())?;
        let revision = store.get_project(project)?.revision;
        store.record_context_item(OperationId::from_bytes([0x61; 16]), project, ContextItemDraft {
            expected_project_revision: revision,
            role: ContextItemRole::Goal,
            statement: format!("Inspect {filename}"),
            provenance_role: StatementProvenanceRole::UserStatement,
            author: Principal { kind: PrincipalKind::User, identity: "authored-test-fixture".into() },
            source_basis: vec![source_id],
            applicability: ApplicabilityScope { paths: vec![filename.into()], components: vec![], work_contexts: vec![] },
        })?;
        drop(store);
        let produced = operations.analyze(project, vec![])?.value.ok_or("analysis missing")?;
        let manifest = fs::read(&produced.stored_at)?;
        let analysis = produced.analysis;
        let mut expected_relations = Vec::new();
        for (caller, resolved) in [(negative_caller, false), ("local", true)] {
            let caller = analysis.structural_facts.iter().find(|f| f.entity.display_name.as_deref() == Some(caller)).ok_or("caller missing")?;
            let relation = analysis.semantic_results.iter().find(|r| r.relation.source_entity == caller.entity.identity && r.relation.kind == SemanticRelationKind::References).ok_or("reference missing")?;
            match (&relation.relation.target, resolved) {
                (RelationTarget::ResolvedEntity(id), true) => assert!(analysis.structural_facts.iter().any(|f| f.entity.identity == *id && f.entity.display_name.as_deref() == Some(expected_name))),
                (RelationTarget::Unresolved(target), false) => assert!(!target.reason.is_empty()),
                _ => return Err(format!("{filename}: incorrect source binding for {:?}", caller.entity.display_name).into()),
            }
            expected_relations.push((relation.clone(), resolved));
        }
        drop(operations);
        let restarted = LocalOperations::new(layout.clone());
        let canonical = restarted.canonical_basis(project)?;
        let canonical_identity = canonical.stable_ordering_identity.clone();
        let grounding = CanonicalGrounding::from_read_basis(&canonical)?;
        let hits = search_local(&analysis, "references", analysis.repository_snapshot, 100, &grounding)?;
        let explanation_basis = grounded_explanation_basis(&analysis, analysis.repository_snapshot, &grounding)?;
        let projection = restarted.project_projection(project)?;
        for (result, resolved) in &expected_relations {
            let relation = &result.relation;
            let hit = hits.iter().find(|h| h.result_kind == SearchResultKind::SemanticRelation && h.source_range == relation.supporting_range && h.relation_target.as_ref() == Some(&relation.target)).ok_or("search lost call meaning")?;
            assert_eq!(hit.semantic_provenance.as_ref(), Some(&result.provenance));
            assert_eq!(hit.uncertainty.level, if *resolved { UncertaintyLevel::None } else { UncertaintyLevel::High });
            let evidence = explanation_basis.evidence.iter().find(|e| e.identity == relation.identity && e.statement_class == GroundingStatementClass::SemanticResult).ok_or("grounding lost call")?;
            assert_eq!(evidence.relation_target.as_ref(), Some(&relation.target));
            assert_eq!(evidence.source_range, hit.source_range);
            let map = projection.current_work_topology.relations.iter().find(|r| r.identity == relation.identity).ok_or("persisted Projection lost call")?;
            assert_eq!(map.class, MapRelationClass::SemanticResult);
            assert_eq!(map.target_entity.is_some(), *resolved);
            assert_eq!(map.unresolved_target.is_some(), !resolved);
            assert_eq!(map.semantic_provenance.as_ref(), Some(&result.provenance));
            assert_eq!(map.supporting_range, relation.supporting_range);
            assert_eq!(map.analysis_snapshot, analysis.identity);
            assert_eq!(map.repository_snapshot, analysis.repository_snapshot);
            assert_eq!(map.freshness.state, FreshnessState::Current);
            assert_eq!(map.uncertainty, relation.uncertainty);
        }
        let mut host = HostAdapter::new(restarted);
        let response = host.handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"repository_understanding","arguments":{"project_id":project.to_string()}}})).ok_or("MCP response")?;
        assert_eq!(response["result"]["isError"], false, "{response}");
        let reading = &response["result"]["structuredContent"];
        for (result, resolved) in &expected_relations {
            let relation = &result.relation;
            let rows = if *resolved { &reading["architecture"]["relationships"] } else { &reading["evidence"]["unresolved_relationships"] };
            let row = rows.as_array().ok_or("reader relationships")?.iter().find(|r| r["identity"] == relation.identity).ok_or_else(|| format!("{filename}: reader omitted call: {reading}"))?;
            assert_eq!(row["analysis_snapshot_id"], analysis.identity.to_string());
            assert_eq!(row["repository_snapshot_id"], analysis.repository_snapshot.to_string());
            assert_eq!(row["semantic_provenance"], serde_json::to_value(&result.provenance)?);
            assert_eq!(row["source_range"], serde_json::to_value(&relation.supporting_range)?);
            if !resolved {
                assert!(row["unresolved_target"].as_str().is_some_and(|s| s.contains("binding")));
                assert_eq!(row["uncertainty"]["level"], "high");
                assert_eq!(row["freshness"]["state"], "current");
                assert!(row["diagnostics"].as_array().is_some_and(|d| !d.is_empty()));
                assert!(!reading["architecture"]["relationships"].as_array().ok_or("architecture rows")?.iter().any(|r| r["identity"] == relation.identity));
            }
            let explanation = reading["deterministic_explanations"].as_array().ok_or("explanations")?.iter().find(|e| e["identity"] == format!("deterministic:relation:{}", relation.identity)).ok_or("reader explanation missing")?;
            if !resolved { assert!(explanation["english"].as_str().is_some_and(|s| s.contains("unresolved"))); }
        }
        assert_eq!(fs::read(&produced.stored_at)?, manifest);
        assert_eq!(host.operations().canonical_basis(project)?.stable_ordering_identity, canonical_identity);
    }
    Ok(())
}
