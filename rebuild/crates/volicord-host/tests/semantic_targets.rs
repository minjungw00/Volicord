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
use volicord_projections::{build_project_understanding, MapRelationClass, UnderstandingBound};
use volicord_repository_intelligence::{
    grounded_explanation_basis, search_local, CanonicalGrounding, FreshnessState,
    GroundingStatementClass, RelationTarget, SearchResultKind, SemanticRelationKind,
    UncertaintyLevel,
};

#[test]
fn ordinary_analysis_preserves_binding_meaning_through_restart_and_public_readers(
) -> Result<(), Box<dyn Error>> {
    // Expected target declaration text is authored from source semantics, not
    // analyzer-qualified names (anonymous methods can share those names).
    type Call<'a> = (&'a str, &'a str, Option<&'a str>);
    type Case<'a> = (&'a str, &'a str, &'a [Call<'a>]);
    let cases: &[Case<'_>] = &[
        ("receiver.ts", include_str!("../../../validation/repository-intelligence/semantic-binding/receiver.ts"), &[
            ("demo", "console.log", None), ("local", "log", Some("function log(")),
        ]),
        ("Owner.java", "class Owner {\nstatic void println(String value) {}\nstatic void demo() { System.out.println(\"host\"); }\nstatic void local() { println(\"local\"); }\n}\n", &[
            ("demo", "System.out.println", None), ("local", "println", Some("static void println(")),
        ]),
        ("lib.rs", "fn len() {}\nfn demo(value: &str) { value.len(); }\nfn local() { len(); }\n", &[
            ("demo", "value.len", None), ("local", "len", Some("fn len(")),
        ]),
        ("objects.ts", "function real(value: number) {}\nfunction orphan() { const object = { run() {} }; run(); }\nfunction local() { real(1); }\n", &[
            ("orphan", "run", None), ("local", "real", Some("function real(")),
        ]),
        ("Outer.java", "class Outer {\nvoid ping() {}\nvoid local() { this.ping(); }\nRunnable create() { return new Runnable() {\nvoid ping() {}\npublic void anonymous() { this.ping(); }\npublic void run() {}\n}; }\n}\n", &[
            ("anonymous", "this.ping", None), ("local", "this.ping", Some("void ping() {}")),
        ]),
        ("OuterLambda.java", "class Outer {\nvoid ping() {}\nvoid local() { Runnable r = () -> this.ping(); }\nRunnable create() { return new Runnable() {\nvoid ping() {}\nvoid anonymousLambda() { Runnable r = () -> this.ping(); }\npublic void run() {}\n}; }\n}\n", &[
            ("anonymousLambda", "this.ping", None), ("local", "this.ping", Some("void ping() {}")),
        ]),
        ("OuterNested.java", "class Outer {\nvoid ping() {}\nRunnable create() { return new Runnable() {\nvoid ping() {}\nvoid anonymous() { this.ping(); }\nclass Inside {\nvoid ping() { int own = 1; }\nvoid namedInsideAnonymous() { this.ping(); }\n}\npublic void run() {}\n}; }\n}\n", &[
            ("anonymous", "this.ping", None), ("namedInsideAnonymous", "this.ping", Some("void ping() { int own = 1; }")),
        ]),
        ("OuterEnum.java", "class Outer {\nvoid ping() {}\nvoid local() { this.ping(); }\nenum Choice {\nONE {\nvoid ping() {}\nvoid enumAnonymous() { this.ping(); }\n};\n}\n}\n", &[
            ("enumAnonymous", "this.ping", None), ("local", "this.ping", Some("void ping() {}")),
        ]),
    ];
    for &(filename, text, calls) in cases {
        let home = tempfile::tempdir()?;
        let repository = home.path().join("repository");
        fs::create_dir(&repository)?;
        fs::write(repository.join(filename), text)?;
        let layout = RuntimeLayout::new(home.path().join("runtime"))?;
        let operations = LocalOperations::new(layout.clone());
        let project = operations
            .initialize_project("Binding reader control", Some(&repository))?
            .project
            .id;
        // Explicit authored Work scope seeds the ordinary architecture reader.
        let source = operations.record_user_source(
            project,
            "test-fixture".into(),
            "semantic-binding-control".into(),
            format!("Inspect {filename}"),
        )?;
        let source_id = operations
            .canonical_basis(project)?
            .sources
            .iter()
            .find(|s| s.source.id.to_string() == source.identity)
            .ok_or("fixture Source")?
            .source
            .id;
        let mut store = Store::open(layout.canonical_store())?;
        let revision = store.get_project(project)?.revision;
        store.record_context_item(
            OperationId::from_bytes([0x61; 16]),
            project,
            ContextItemDraft {
                expected_project_revision: revision,
                role: ContextItemRole::Goal,
                statement: format!("Inspect {filename}"),
                provenance_role: StatementProvenanceRole::UserStatement,
                author: Principal {
                    kind: PrincipalKind::User,
                    identity: "authored-test-fixture".into(),
                },
                source_basis: vec![source_id],
                applicability: ApplicabilityScope {
                    paths: vec![filename.into()],
                    components: vec![],
                    work_contexts: vec![],
                },
            },
        )?;
        drop(store);
        let produced = operations
            .analyze(project, vec![])?
            .value
            .ok_or("analysis missing")?;
        let manifest = fs::read(&produced.stored_at)?;
        let analysis = produced.analysis;
        let mut expected_relations = Vec::new();
        for &(caller, spelling, declaration) in calls {
            let caller = analysis
                .structural_facts
                .iter()
                .find(|f| f.entity.display_name.as_deref() == Some(caller))
                .ok_or("caller missing")?;
            let references: Vec<_> = analysis
                .semantic_results
                .iter()
                .filter(|r| {
                    r.relation.source_entity == caller.entity.identity
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
            assert_eq!(
                references.len(),
                if declaration.is_some() { 2 } else { 1 },
                "{filename}:{:?}:{spelling}",
                caller.entity.display_name
            );
            for relation in references {
                match (&relation.relation.target, declaration) {
                    (RelationTarget::ResolvedEntity(id), Some(declaration)) => {
                        let target = analysis
                            .structural_facts
                            .iter()
                            .find(|f| f.entity.identity == *id)
                            .ok_or("target missing")?;
                        let range = target.entity.source_range.as_ref().ok_or("target range")?;
                        assert!(
                            text.lines()
                                .nth(range.start.line as usize)
                                .is_some_and(|line| line.trim_start().starts_with(declaration)),
                            "{filename}:{:?}: incorrect target {:?}",
                            caller.entity.display_name,
                            target.entity
                        );
                    }
                    (RelationTarget::Unresolved(target), None) => {
                        assert!(!target.reason.is_empty())
                    }
                    _ => {
                        return Err(format!(
                            "{filename}: incorrect source binding for {:?}: {:?}",
                            caller.entity.display_name, relation.relation
                        )
                        .into())
                    }
                }
                expected_relations.push((relation.clone(), declaration.is_some()));
            }
        }
        drop(operations);
        let restarted = LocalOperations::new(layout.clone());
        let canonical = restarted.canonical_basis(project)?;
        let canonical_identity = canonical.stable_ordering_identity.clone();
        let grounding = CanonicalGrounding::from_read_basis(&canonical)?;
        let mut hits = search_local(
            &analysis,
            "references",
            analysis.repository_snapshot,
            100,
            &grounding,
        )?;
        hits.extend(search_local(
            &analysis,
            "resolves_to",
            analysis.repository_snapshot,
            100,
            &grounding,
        )?);
        let explanation_basis =
            grounded_explanation_basis(&analysis, analysis.repository_snapshot, &grounding)?;
        let projection = restarted.project_projection(project)?;
        for (result, resolved) in &expected_relations {
            let relation = &result.relation;
            let hit = hits
                .iter()
                .find(|h| {
                    h.result_kind == SearchResultKind::SemanticRelation
                        && h.semantic_provenance == Some(result.provenance.clone())
                        && h.source_range == relation.supporting_range
                        && h.relation_target.as_ref() == Some(&relation.target)
                })
                .ok_or("search lost call meaning")?;
            assert_eq!(hit.semantic_provenance.as_ref(), Some(&result.provenance));
            assert_eq!(
                hit.uncertainty.level,
                if *resolved {
                    UncertaintyLevel::None
                } else {
                    UncertaintyLevel::High
                }
            );
            let evidence = explanation_basis
                .evidence
                .iter()
                .find(|e| {
                    e.identity == relation.identity
                        && e.statement_class == GroundingStatementClass::SemanticResult
                })
                .ok_or("grounding lost call")?;
            assert_eq!(evidence.relation_target.as_ref(), Some(&relation.target));
            assert_eq!(evidence.source_range, hit.source_range);
            let map = projection
                .current_work_topology
                .relations
                .iter()
                .find(|r| r.identity == relation.identity)
                .ok_or("persisted Projection lost call")?;
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
        let complete = build_project_understanding(
            &projection,
            UnderstandingBound {
                max_items_per_section: 1000,
            },
        );
        let mut host = HostAdapter::new(restarted);
        let response = host.handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"repository_understanding","arguments":{"project_id":project.to_string()}}})).ok_or("MCP response")?;
        assert_eq!(response["result"]["isError"], false, "{response}");
        let reading = &response["result"]["structuredContent"];
        for (result, resolved) in &expected_relations {
            let relation = &result.relation;
            let rows = if *resolved {
                &reading["architecture"]["relationships"]
            } else {
                &reading["evidence"]["unresolved_relationships"]
            };
            let complete_explanation = complete
                .deterministic_explanations
                .iter()
                .find(|e| e.identity == format!("deterministic:relation:{}", relation.identity))
                .ok_or("complete explanation missing")?;
            if !resolved {
                assert!(complete_explanation.english.contains("unresolved"));
            }
            let row = rows
                .as_array()
                .ok_or("reader relationships")?
                .iter()
                .find(|r| r["identity"] == relation.identity);
            let Some(row) = row else {
                // Supported readers are bounded. An omitted positive must carry
                // a real section omission; the unresolved counterexample is required.
                assert!(*resolved, "{filename}: unresolved call omitted");
                assert!(reading["omissions"]
                    .as_array()
                    .ok_or("omissions")?
                    .iter()
                    .any(|o| o["section"] == "architecture.relationships"
                        && o["omitted_count"].as_u64().is_some_and(|n| n > 0)));
                continue;
            };
            assert_eq!(row["analysis_snapshot_id"], analysis.identity.to_string());
            assert_eq!(
                row["repository_snapshot_id"],
                analysis.repository_snapshot.to_string()
            );
            assert_eq!(
                row["semantic_provenance"],
                serde_json::to_value(&result.provenance)?
            );
            assert_eq!(
                row["source_range"],
                serde_json::to_value(&relation.supporting_range)?
            );
            if !resolved {
                assert!(row["unresolved_target"]
                    .as_str()
                    .is_some_and(|s| s.contains("binding")));
                assert_eq!(row["uncertainty"]["level"], "high");
                assert_eq!(row["freshness"]["state"], "current");
                assert!(row["diagnostics"].as_array().is_some_and(|d| !d.is_empty()));
                assert!(!reading["architecture"]["relationships"]
                    .as_array()
                    .ok_or("architecture rows")?
                    .iter()
                    .any(|r| r["identity"] == relation.identity));
            }
            let explanation = reading["deterministic_explanations"]
                .as_array()
                .ok_or("explanations")?
                .iter()
                .find(|e| e["identity"] == format!("deterministic:relation:{}", relation.identity));
            if let Some(explanation) = explanation {
                if !resolved {
                    assert!(explanation["english"]
                        .as_str()
                        .is_some_and(|s| s.contains("unresolved")));
                }
            } else {
                assert!(
                    filename != "Outer.java",
                    "primary receiver explanation omitted"
                );
                assert!(reading["omissions"]
                    .as_array()
                    .ok_or("omissions")?
                    .iter()
                    .any(|o| o["section"] == "deterministic_explanations"
                        && o["omitted_count"].as_u64().is_some_and(|n| n > 0)));
            }
        }
        assert_eq!(fs::read(&produced.stored_at)?, manifest);
        assert_eq!(
            host.operations()
                .canonical_basis(project)?
                .stable_ordering_identity,
            canonical_identity
        );
    }
    Ok(())
}
