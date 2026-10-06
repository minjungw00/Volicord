use std::fs;
use tempfile::tempdir;
use volicord_context::{
    ApplicabilityScope, Availability, CanonicalReadOptions, CheckpointDraft, CheckpointKind,
    ContextItemDraft, ContextItemRole, OperationId, Principal, PrincipalKind, SourceDraft,
    SourcePayload, StatementProvenanceRole, Store, UserAcceptanceFact, UserAcceptanceState,
    UserReviewFact, UserReviewState, WorkState,
};
use volicord_inquiry::CandidateStore;
use volicord_projections::{
    build_project_projection, build_project_understanding, CandidateContentAccess,
    CandidateProjectionInput, ProjectProjection, ProjectProjectionInputs, ProjectionBound,
    UnderstandingBound, UnderstandingExplanationKind,
};
use volicord_repository_intelligence::{
    analyze_repository_semantics, AnalysisProvenance, AnalysisSnapshot, AnalyzerIdentity,
    CanonicalGrounding, CodeEntityKind, InventoryRequest, Language, ProvenanceClass,
    RelationTarget, SemanticAnalysisRequest, SemanticAnalysisResult, SemanticProvenance,
    SemanticRelation, SemanticRelationKind, StructuralAnalysisRequest, StructuralRelationKind,
};

fn operation(value: u8) -> OperationId {
    OperationId::from_bytes([value; 16])
}

fn preserve_limitation_control(
    name: &str,
    projection: &ProjectProjection,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(root) = std::env::var_os("VOLICORD_LIMITATION_CONTROL_DIR") {
        let root = std::path::PathBuf::from(root);
        if !root.is_absolute() {
            return Err("limitation control directory must be absolute and ignored".into());
        }
        fs::create_dir_all(&root)?;
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join(format!("{name}.json")))?;
        serde_json::to_writer_pretty(
            file,
            &serde_json::json!({
                "before_basis":"same authored fixture through baseline whole-repository limitation aggregation; not a baseline executable capture",
                "before_issues":format!("{:?}",projection.issues),
                "before_known_limits":projection.resume.known_limits,
                "after_answer_issues":format!("{:?}",projection.answer_issues),
                "after_answer_gaps":format!("{:?}",projection.answer_capability_gaps),
                "retained_repository_gaps":format!("{:?}",projection.repository_map.gaps),
                "qualification":"independent focused relevance control; no human-quality qualification"
            }),
        )?;
    }
    Ok(())
}

fn build_projection(
    files: &[(&str, &str)],
    changed_path: &str,
    limit: usize,
) -> Result<
    (
        ProjectProjection,
        volicord_repository_intelligence::AnalysisSnapshot,
    ),
    Box<dyn std::error::Error>,
> {
    build_projection_scenario(files, &[changed_path], &[vec![changed_path]], limit, |_| {
        Ok(())
    })
}

fn build_projection_scenario<F>(
    files: &[(&str, &str)],
    goal_paths: &[&str],
    checkpoint_paths: &[Vec<&str>],
    limit: usize,
    enrich_analysis: F,
) -> Result<(ProjectProjection, AnalysisSnapshot), Box<dyn std::error::Error>>
where
    F: FnOnce(&mut AnalysisSnapshot) -> Result<(), Box<dyn std::error::Error>>,
{
    let (canonical, analysis, candidates) =
        projection_scenario_basis(files, goal_paths, checkpoint_paths, enrich_analysis)?;
    let project_id = canonical.project.id;
    let projection = build_project_projection(ProjectProjectionInputs {
        requirements: volicord_projections::ProjectionReadRequirements::default(),
        metadata: &[],
        detail: volicord_projections::ProjectionDetail::default(),
        selection: volicord_projections::WorkSelector::LatestWork,
        analysis_issues: &[],
        canonical: &canonical,
        analyses: &[&analysis],
        applicability: volicord_inquiry::ApplicabilityQuery {
            project_id,
            paths: goal_paths.iter().map(|path| (*path).into()).collect(),
            components: Vec::new(),
            work_contexts: vec!["viewer-current-work".into()],
            current_assumptions: Vec::new(),
            met_revisit_triggers: Vec::new(),
        },
        candidates: CandidateProjectionInput::Available(&candidates),
        candidate_content_access: CandidateContentAccess::PolicyWithheld,
        observed_at: canonical.project.updated_at,
        bound: ProjectionBound {
            max_items_per_section: limit,
        },
    })
    .expect("valid default selection");
    Ok((projection, analysis))
}

fn projection_scenario_basis<F>(
    files: &[(&str, &str)],
    goal_paths: &[&str],
    checkpoint_paths: &[Vec<&str>],
    enrich_analysis: F,
) -> Result<
    (
        volicord_context::CanonicalReadBasis,
        AnalysisSnapshot,
        volicord_inquiry::CandidateReadBasis,
    ),
    Box<dyn std::error::Error>,
>
where
    F: FnOnce(&mut AnalysisSnapshot) -> Result<(), Box<dyn std::error::Error>>,
{
    let temporary = tempdir()?;
    let repository = temporary.path().join("repository");
    fs::create_dir_all(&repository)?;
    for (path, content) in files {
        let destination = repository.join(path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(destination, content)?;
    }

    let mut store = Store::open(temporary.path().join("canonical.sqlite3"))?;
    let project = store
        .create_project(operation(1), "Current work flow")?
        .value;
    let repository_source = store
        .record_source(
            operation(2),
            project.id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::RepositorySnapshot {
                    revision: "current-work-fixture".into(),
                },
                actor: Principal {
                    kind: PrincipalKind::Repository,
                    identity: "fixture".into(),
                },
                observer: None,
                availability: Availability::Available,
            },
        )?
        .value;
    let user_turn = store
        .record_source(
            operation(3),
            project.id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::CurrentHostUserTurn {
                    host: "projection-test".into(),
                    session: "current-work-flow".into(),
                    turn: format!("Continue work in {}", goal_paths.join(", ")),
                },
                actor: Principal {
                    kind: PrincipalKind::User,
                    identity: "owner".into(),
                },
                observer: None,
                availability: Availability::Available,
            },
        )?
        .value;
    let goal = store
        .record_context_item(
            operation(4),
            project.id,
            ContextItemDraft {
                expected_project_revision: project.revision,
                role: ContextItemRole::Goal,
                statement: "Keep the current work flow inspectable".into(),
                provenance_role: StatementProvenanceRole::UserStatement,
                author: Principal {
                    kind: PrincipalKind::User,
                    identity: "owner".into(),
                },
                source_basis: vec![user_turn.id],
                applicability: ApplicabilityScope {
                    paths: goal_paths.iter().map(|path| (*path).into()).collect(),
                    components: Vec::new(),
                    work_contexts: vec!["viewer-current-work".into()],
                },
            },
        )?
        .value;
    for (index, paths) in checkpoint_paths.iter().enumerate() {
        store.record_checkpoint(
            operation(5 + u8::try_from(index)?),
            project.id,
            CheckpointDraft {
                expected_project_revision: project.revision,
                work_item_id: Some(goal.id),
                kind: CheckpointKind::Handoff,
                goal: "Keep the current work flow inspectable".into(),
                work_state: WorkState::Paused,
                state_change: Some(if paths.is_empty() {
                    "Verification completed without additional source changes".into()
                } else {
                    "Repository Intelligence analysis is available".into()
                }),
                source_basis: vec![repository_source.id, user_turn.id],
                changed_source_basis: vec![repository_source.id],
                changed_paths: paths.iter().map(|path| (*path).into()).collect(),
                applied_decisions: Vec::new(),
                verification: Vec::new(),
                user_review: UserReviewFact {
                    state: UserReviewState::NotRequested,
                    source_id: None,
                },
                user_acceptance: UserAcceptanceFact {
                    state: UserAcceptanceState::NotRequested,
                    source_id: None,
                },
                known_limits: Vec::new(),
                non_goals: Vec::new(),
                open_questions: Vec::new(),
                next_step: "Inspect the grounded current-work relation".into(),
                handoff_to: Some("next agent".into()),
            },
        )?;
    }
    let canonical = store.read_canonical_basis(
        project.id,
        CanonicalReadOptions {
            include_checkpoint_history: true,
        },
    )?;
    let grounding = CanonicalGrounding::from_read_basis(&canonical)?;
    let (_, mut analysis) = analyze_repository_semantics(SemanticAnalysisRequest::new(
        StructuralAnalysisRequest::new(InventoryRequest::new(
            &repository,
            &grounding,
            repository_source.id,
            1,
        )?),
    ))?;
    enrich_analysis(&mut analysis)?;
    let candidates = CandidateStore::open(temporary.path().join("candidates.sqlite3"))?
        .read_basis(project.id)?;
    Ok((canonical, analysis, candidates))
}

fn add_fixture_flow_relation(
    analysis: &mut AnalysisSnapshot,
    identity: &str,
    source_path: &str,
    target_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = analysis
        .structural_facts
        .iter()
        .find(|fact| {
            fact.entity.area.path == source_path && fact.entity.kind == CodeEntityKind::File
        })
        .map(|fact| fact.entity.clone())
        .ok_or_else(|| format!("fixture file entity missing: {source_path}"))?;
    let target_identity = analysis
        .structural_facts
        .iter()
        .find(|fact| {
            fact.entity.area.path == target_path && fact.entity.kind == CodeEntityKind::File
        })
        .map(|fact| fact.entity.identity.clone())
        .ok_or_else(|| format!("fixture file entity missing: {target_path}"))?;
    let adapter = source
        .source_range
        .as_ref()
        .map(|range| range.adapter.clone())
        .ok_or_else(|| format!("fixture file Source range missing: {source_path}"))?;
    let analyzer = AnalyzerIdentity {
        name: "current-work-flow-fixture".into(),
        version: "1".into(),
    };
    analysis.semantic_results.push(SemanticAnalysisResult {
        relation: SemanticRelation {
            identity: identity.into(),
            repository_snapshot: analysis.repository_snapshot,
            analysis_snapshot: analysis.identity,
            source_entity: source.identity,
            target: RelationTarget::ResolvedEntity(target_identity),
            kind: SemanticRelationKind::References,
            supporting_range: source.source_range,
            diagnostics: Vec::new(),
            uncertainty: source.uncertainty,
            freshness: source.freshness,
            extensions: Vec::new(),
        },
        provenance: SemanticProvenance {
            adapter: adapter.clone(),
            analyzer: analyzer.clone(),
            build_context: Some("explicit polyglot binding-flow fixture".into()),
            resolution_basis: "fixture-supplied resolved endpoint identities".into(),
            analysis: AnalysisProvenance {
                class: ProvenanceClass::SemanticResult,
                repository_snapshot: analysis.repository_snapshot,
                analysis_snapshot: analysis.identity,
                adapter: Some(adapter),
                analyzer: Some(analyzer),
                source_basis: vec![source.source],
                observed_or_generated_at_unix_micros: analysis.generated_at_unix_micros,
            },
        },
    });
    Ok(())
}

#[test]
fn small_python_flow_survives_before_generic_repository_map_bounds(
) -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
def format_name(name: str) -> str:
    return name.strip()

class Greeter:
    def greet(self, person: str) -> str:
        return format_name(person)
"#;
    let (projection, analysis) =
        build_projection(&[("src/greeter/core.py", source)], "src/greeter/core.py", 2)?;
    let selected = projection
        .current_work_topology
        .relations
        .iter()
        .find(|relation| relation.kind == "CallsSyntactically")
        .ok_or("current-work Python call evidence was lost before Project Understanding")?;
    let raw_call = analysis
        .structural_facts
        .iter()
        .flat_map(|fact| &fact.relations)
        .find(|relation| relation.identity == selected.identity)
        .ok_or("selected call is not backed by the Analysis Snapshot")?;
    assert_eq!(raw_call.kind, StructuralRelationKind::CallsSyntactically);

    // Diagnostic chain: RI has an unresolved-but-real call from a path-located
    // entity. The generic map bound discards that source entity and call, which
    // was the first old loss point. Current-work selection retains the honest
    // unresolved relation before that bound and never invents a target entity.
    assert!(analysis.structural_facts.iter().any(|fact| {
        fact.entity.identity == raw_call.source_entity
            && fact.entity.area.path == "src/greeter/core.py"
    }));
    assert!(!projection
        .repository_map
        .relations
        .iter()
        .any(|relation| relation.identity == raw_call.identity));
    assert_eq!(selected.source_entity, raw_call.source_entity);
    assert!(selected.target_entity.is_none());
    assert!(selected.unresolved_target.is_some());
    assert!(projection.current_work_code.iter().any(|link| {
        link.entity_identity == selected.source_entity
            && link.changed_paths == ["src/greeter/core.py"]
    }));

    let understanding = build_project_understanding(
        &projection,
        UnderstandingBound {
            max_items_per_section: 2,
        },
    );
    assert!(understanding
        .evidence
        .unresolved_relationships
        .iter()
        .any(|relation| relation.identity == raw_call.identity));
    let visible = understanding
        .architecture
        .components
        .iter()
        .map(|entity| entity.identity.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(visible.contains(raw_call.source_entity.as_str()));
    assert!(understanding
        .deterministic_explanations
        .iter()
        .any(|explanation| {
            explanation.kind == UnderstandingExplanationKind::Flow
                && explanation.relation_basis.contains(&raw_call.identity)
        }));
    Ok(())
}

#[test]
fn resolved_current_work_relation_keeps_both_endpoints_under_the_bound(
) -> Result<(), Box<dyn std::error::Error>> {
    let (projection, analysis) = build_projection(
        &[(
            "src/worker.py",
            "class Worker:\n    def run(self) -> str:\n        return 'ok'\n",
        )],
        "src/worker.py",
        2,
    )?;
    let selected = projection
        .current_work_topology
        .relations
        .iter()
        .find(|relation| {
            relation.target_entity.is_some()
                && matches!(relation.kind.as_str(), "Contains" | "Declares")
        })
        .ok_or("resolved current-work containment relation missing")?;
    assert!(analysis
        .structural_facts
        .iter()
        .flat_map(|fact| &fact.relations)
        .any(|relation| relation.identity == selected.identity));
    let target = selected
        .target_entity
        .as_deref()
        .expect("selected relation is resolved");
    let retained = projection
        .current_work_topology
        .entities
        .iter()
        .map(|entity| entity.identity.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(retained.contains(selected.source_entity.as_str()));
    assert!(retained.contains(target));

    let understanding = build_project_understanding(
        &projection,
        UnderstandingBound {
            max_items_per_section: 2,
        },
    );
    let relation = understanding
        .architecture
        .relationships
        .iter()
        .find(|relation| relation.identity == selected.identity)
        .ok_or("Project Understanding dropped the bounded resolved relation")?;
    let visible = understanding
        .architecture
        .components
        .iter()
        .map(|entity| entity.identity.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(visible.contains(relation.source_entity.as_str()));
    assert!(relation
        .target_entity
        .as_deref()
        .is_some_and(|target| visible.contains(target)));
    Ok(())
}

#[test]
fn polyglot_keeps_real_local_flow_without_fabricating_a_cross_language_edge(
) -> Result<(), Box<dyn std::error::Error>> {
    let (projection, analysis) = build_projection(
        &[
            (
                "python/formatter.py",
                "def format_greeting(name: str) -> str:\n    return f'hello, {name.strip()}'\n",
            ),
            (
                "typescript/client.ts",
                "export function readReply(message: string): string { return message; }\n",
            ),
            (
                "system.json",
                "{\"formatter\":\"python/formatter.py\",\"client\":\"typescript/client.ts\"}\n",
            ),
        ],
        "python/formatter.py",
        8,
    )?;
    assert!(analysis
        .structural_facts
        .iter()
        .any(|fact| fact.entity.language == Language::Python));
    assert!(analysis
        .structural_facts
        .iter()
        .any(|fact| fact.entity.language == Language::TypeScript));
    let entities = projection
        .current_work_topology
        .entities
        .iter()
        .map(|entity| (entity.identity.as_str(), &entity.language))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert!(!projection
        .current_work_topology
        .relations
        .iter()
        .any(|relation| {
            relation.target_entity.as_deref().is_some_and(|target| {
                entities
                    .get(relation.source_entity.as_str())
                    .zip(entities.get(target))
                    .is_some_and(|(source_language, target_language)| {
                        source_language != target_language
                    })
            })
        }));
    let local_call = projection
        .current_work_topology
        .relations
        .iter()
        .find(|relation| relation.kind == "CallsSyntactically")
        .ok_or("actual Python call evidence missing from polyglot current work")?;
    assert!(local_call.target_entity.is_none());
    assert!(local_call
        .unresolved_target
        .as_deref()
        .is_some_and(|reason| reason.contains("structural analysis does not resolve")));
    let understanding = build_project_understanding(
        &projection,
        UnderstandingBound {
            max_items_per_section: 8,
        },
    );
    let flow = understanding
        .deterministic_explanations
        .iter()
        .find(|explanation| {
            explanation.kind == UnderstandingExplanationKind::Flow
                && explanation.relation_basis.contains(&local_call.identity)
        })
        .ok_or("grounded local polyglot flow explanation missing")?;
    assert!(flow
        .known_gaps
        .iter()
        .any(|gap| gap.contains("unresolved") && gap.contains("not a Code Entity")));
    Ok(())
}

#[test]
fn polyglot_current_work_keeps_same_work_history_flow_grounding(
) -> Result<(), Box<dyn std::error::Error>> {
    const NATIVE_TO_BOUNDARY: &str = "fixture-flow:native-to-runtime-boundary";
    const BOUNDARY_TO_CONSUMER: &str = "fixture-flow:runtime-boundary-to-consumer";
    let work_paths = [
        "native/query.c",
        "runtime/query_boundary.ts",
        "typescript/query.ts",
    ];
    let (projection, analysis) = build_projection_scenario(
        &[
            (
                work_paths[0],
                "void collect_query_properties(void *query) { /* native directive maps */ }\n",
            ),
            (
                work_paths[1],
                "export function readPropertyMaps(nativeResult: unknown) { return nativeResult; }\n",
            ),
            (
                work_paths[2],
                "import { readPropertyMaps } from '../runtime/query_boundary';\nexport function matches(nativeResult: unknown) { return readPropertyMaps(nativeResult); }\n",
            ),
        ],
        &[],
        &[work_paths.to_vec(), Vec::new()],
        64,
        |analysis| {
            // The fixture Snapshot explicitly supplies the cross-language
            // relations. The projection must preserve these records; it must
            // never infer equivalent edges merely from adjacent file names.
            add_fixture_flow_relation(
                analysis,
                NATIVE_TO_BOUNDARY,
                work_paths[0],
                work_paths[1],
            )?;
            add_fixture_flow_relation(
                analysis,
                BOUNDARY_TO_CONSUMER,
                work_paths[1],
                work_paths[2],
            )
        },
    )?;

    let raw_results = analysis
        .semantic_results
        .iter()
        .filter(|relation| {
            matches!(
                relation.relation.identity.as_str(),
                NATIVE_TO_BOUNDARY | BOUNDARY_TO_CONSUMER
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(raw_results.len(), 2);
    assert!(raw_results.iter().all(|result| {
        result.provenance.analysis.class == ProvenanceClass::SemanticResult
            && result.relation.analysis_snapshot == analysis.identity
            && result.relation.repository_snapshot == analysis.repository_snapshot
            && matches!(result.relation.target, RelationTarget::ResolvedEntity(_))
    }));
    let entity_paths = analysis
        .structural_facts
        .iter()
        .map(|fact| {
            (
                fact.entity.identity.as_str(),
                fact.entity.area.path.as_str(),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let directed_paths = raw_results
        .iter()
        .filter_map(|result| {
            let RelationTarget::ResolvedEntity(target) = &result.relation.target else {
                return None;
            };
            Some((
                *entity_paths.get(result.relation.source_entity.as_str())?,
                *entity_paths.get(target.as_str())?,
            ))
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        directed_paths,
        [
            (work_paths[0], work_paths[1]),
            (work_paths[1], work_paths[2]),
        ]
        .into_iter()
        .collect()
    );

    // The latest Checkpoint intentionally records verification with no new
    // paths. Earlier paths belong to the same stable Work Item and remain the
    // meaningful grounding basis for this current-work source flow.
    let projected_relations = projection
        .current_work_topology
        .relations
        .iter()
        .map(|relation| relation.identity.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        projected_relations.contains(NATIVE_TO_BOUNDARY),
        "native boundary relation missing from {projected_relations:?}"
    );
    assert!(
        projected_relations.contains(BOUNDARY_TO_CONSUMER),
        "consumer relation missing from {projected_relations:?}"
    );
    assert!(work_paths.iter().all(|path| {
        projection
            .current_work_code
            .iter()
            .any(|link| link.changed_paths.iter().any(|changed| changed == path))
    }));

    let understanding = build_project_understanding(
        &projection,
        UnderstandingBound {
            max_items_per_section: 64,
        },
    );
    let explained_relations = understanding
        .deterministic_explanations
        .iter()
        .filter(|explanation| explanation.kind == UnderstandingExplanationKind::Relationship)
        .flat_map(|explanation| explanation.relation_basis.iter().map(String::as_str))
        .collect::<std::collections::BTreeSet<_>>();
    assert!(explained_relations.contains(NATIVE_TO_BOUNDARY));
    assert!(explained_relations.contains(BOUNDARY_TO_CONSUMER));
    Ok(())
}

#[test]
fn work_limitations_require_actual_affected_inventory_scope(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_repository_intelligence::{Capability, CapabilityState};
    let (projection, analysis) = build_projection_scenario(
        &[
            ("src/lib.rs", "pub fn work() {}"),
            ("other/query.cpp", "void query() {}"),
        ],
        &["src/lib.rs"],
        &[vec!["src/lib.rs"]],
        64,
        |analysis| {
            let report = analysis
                .capabilities
                .iter_mut()
                .find(|r| {
                    r.language == Some(Language::Rust) && r.capability == Capability::Structural
                })
                .ok_or("missing Rust structural capability")?;
            report.state = CapabilityState::Failed;
            report.reason = Some("selected Rust parser failed".into());
            report.user_visible_consequence = Some("Work declarations cannot be verified".into());
            report.usable_remainder = Some("Inventory and canonical Work remain usable".into());
            Ok(())
        },
    )?;
    assert!(analysis
        .capabilities
        .iter()
        .any(|r| r.language == Some(Language::Cpp) && r.state != CapabilityState::Available));
    assert!(projection
        .repository_map
        .gaps
        .iter()
        .any(|g| g.language == Some(Language::Cpp)));
    assert!(projection
        .answer_capability_gaps
        .iter()
        .all(|g| g.language != Some(Language::Cpp)));
    preserve_limitation_control("relevant-rust-unrelated-cpp", &projection)?;
    let failed = projection
        .answer_capability_gaps
        .iter()
        .find(|g| g.reason == "selected Rust parser failed")
        .ok_or("relevant failure hidden")?;
    assert_eq!(failed.state, CapabilityState::Failed);
    assert_eq!(
        failed.user_visible_consequence.as_deref(),
        Some("Work declarations cannot be verified")
    );
    assert_eq!(
        failed.usable_remainder.as_deref(),
        Some("Inventory and canonical Work remain usable")
    );
    let understanding = build_project_understanding(&projection, UnderstandingBound::default());
    assert!(understanding
        .architecture
        .gaps
        .iter()
        .all(|g| g.language != Some(Language::Cpp)));
    Ok(())
}

#[test]
fn same_language_failure_outside_work_does_not_limit_selected_answer(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_repository_intelligence::{AreaId, AreaKind, Capability, CapabilityState};
    let (projection, _) = build_projection_scenario(
        &[
            ("src/lib.rs", "pub fn work() {}"),
            ("unrelated/broken.rs", "pub fn other() {}"),
        ],
        &["src/lib.rs"],
        &[vec!["src/lib.rs"]],
        64,
        |analysis| {
            let report = analysis
                .capabilities
                .iter_mut()
                .find(|r| {
                    r.language == Some(Language::Rust) && r.capability == Capability::Structural
                })
                .ok_or("missing Rust structural capability")?;
            report.state = CapabilityState::Partial;
            report.reason = Some("unrelated file failed".into());
            report.coverage.failed = vec![AreaId {
                kind: AreaKind::File,
                path: "unrelated/broken.rs".into(),
            }];
            Ok(())
        },
    )?;
    assert!(projection
        .repository_map
        .gaps
        .iter()
        .any(|g| g.reason == "unrelated file failed"));
    assert!(!projection
        .answer_capability_gaps
        .iter()
        .any(|g| g.reason == "unrelated file failed"));
    preserve_limitation_control("unrelated-same-language-failure", &projection)?;
    Ok(())
}

#[test]
fn analysis_status_distinguishes_current_coverage_from_partial_failed_and_unknown(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_projections::RepositoryAnalysisState;
    use volicord_repository_intelligence::{Capability, CapabilityState, FreshnessState};
    for (changed_capability, state, freshness, expected) in [
        (
            Capability::Structural,
            CapabilityState::Available,
            FreshnessState::Current,
            RepositoryAnalysisState::Current,
        ),
        (
            Capability::Structural,
            CapabilityState::Partial,
            FreshnessState::Current,
            RepositoryAnalysisState::Partial,
        ),
        (
            Capability::Structural,
            CapabilityState::Failed,
            FreshnessState::Current,
            RepositoryAnalysisState::Failed,
        ),
        (
            Capability::Structural,
            CapabilityState::Available,
            FreshnessState::Stale,
            RepositoryAnalysisState::Stale,
        ),
        (
            Capability::Structural,
            CapabilityState::Available,
            FreshnessState::Unknown,
            RepositoryAnalysisState::FreshnessUnknown,
        ),
    ] {
        let (projection, _) = build_projection_scenario(
            &[("src/lib.rs", "pub fn work() {}")],
            &["src/lib.rs"],
            &[vec!["src/lib.rs"]],
            64,
            |a| {
                for report in &mut a.capabilities {
                    report.state = CapabilityState::Available;
                }
                let report = a
                    .capabilities
                    .iter_mut()
                    .find(|r| {
                        r.capability == changed_capability && r.language == Some(Language::Rust)
                    })
                    .ok_or("structural Rust")?;
                report.state = state;
                a.freshness.state = freshness;
                Ok(())
            },
        )?;
        assert_eq!(projection.repository_analysis.state, expected);
        assert_eq!(
            projection
                .repository_analysis
                .freshness
                .as_ref()
                .ok_or("freshness")?
                .state,
            freshness
        );
    }
    Ok(())
}

#[test]
fn shared_repository_source_does_not_seed_unrelated_work_components(
) -> Result<(), Box<dyn std::error::Error>> {
    let (projection, _) = build_projection(
        &[
            ("src/lib.rs", "pub fn work() {}"),
            ("unrelated/other.rs", "pub fn other() {}"),
        ],
        "src/lib.rs",
        64,
    )?;
    assert!(projection
        .repository_map
        .entities
        .iter()
        .any(|e| e.locator == "unrelated/other.rs"));
    assert!(projection
        .current_work_topology
        .entities
        .iter()
        .all(|e| e.locator != "unrelated/other.rs"));
    assert!(projection.current_work_code.iter().all(|link| !projection
        .repository_map
        .entities
        .iter()
        .any(|e| e.identity == link.entity_identity && e.locator == "unrelated/other.rs")));
    let understanding = build_project_understanding(&projection, UnderstandingBound::default());
    assert!(understanding
        .architecture
        .components
        .iter()
        .all(|e| e.locator != "unrelated/other.rs"));
    Ok(())
}

#[test]
fn reference_and_containment_evidence_never_claims_execution_flow(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_projections::{ArchitectureFlowState, CodeRelationshipRole};
    let (projection, _) = build_projection_scenario(
        &[
            ("src/lib.rs", "pub fn work() {}"),
            ("src/neighbor.rs", "pub fn neighbor() {}"),
        ],
        &["src/lib.rs"],
        &[vec!["src/lib.rs"]],
        64,
        |a| add_fixture_flow_relation(a, "fixture-reference", "src/lib.rs", "src/neighbor.rs"),
    )?;
    let understanding = build_project_understanding(&projection, UnderstandingBound::default());
    assert!(understanding
        .architecture
        .relationships
        .iter()
        .any(|r| r.identity == "fixture-reference"
            && r.role() == CodeRelationshipRole::SymbolReference));
    assert_eq!(
        understanding.architecture.flow_evidence.state,
        ArchitectureFlowState::NoResolvedCalls
    );
    assert!(understanding
        .architecture
        .flow_evidence
        .relation_ids
        .is_empty());
    assert!(understanding
        .architecture
        .flow_evidence
        .missing_evidence
        .iter()
        .any(|s| s.contains("CallsSyntactically")));
    assert!(!understanding
        .deterministic_explanations
        .iter()
        .any(|e| e.kind == UnderstandingExplanationKind::Flow
            && e.relation_basis.iter().any(|id| id == "fixture-reference")));
    Ok(())
}

#[test]
fn relevant_unavailable_capability_retains_unknown_freshness_separately(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_repository_intelligence::{Capability, CapabilityState, FreshnessState};
    let (projection, _) = build_projection_scenario(
        &[("src/lib.rs", "pub fn work() {}")],
        &["src/lib.rs"],
        &[vec!["src/lib.rs"]],
        64,
        |a| {
            a.freshness.state = FreshnessState::Unknown;
            let r = a
                .capabilities
                .iter_mut()
                .find(|r| {
                    r.language == Some(Language::Rust) && r.capability == Capability::Structural
                })
                .ok_or("Rust structural")?;
            r.state = CapabilityState::Unavailable;
            r.freshness.state = FreshnessState::Unknown;
            r.reason = Some("current repository comparison unavailable".into());
            Ok(())
        },
    )?;
    let gap = projection
        .answer_capability_gaps
        .iter()
        .find(|g| g.capability == Capability::Structural)
        .ok_or("related unknown freshness hidden")?;
    assert_eq!(gap.state, CapabilityState::Unavailable);
    assert_eq!(gap.freshness.state, FreshnessState::Unknown);
    assert_eq!(gap.reason, "current repository comparison unavailable");
    Ok(())
}

#[test]
fn exact_decision_limits_survive_unrelated_checkpoint_and_supersession(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_context::{
        AgentRecommendation, Decision, DecisionChoice, DecisionId, DecisionLifecycle,
        DecisionWorkScope, QuestionId,
    };
    use volicord_projections::{ProjectionDetail, ProjectionReadRequirements, WorkSelector};
    use volicord_repository_intelligence::{Capability, CapabilityState};

    let (mut canonical, analysis, candidates) = projection_scenario_basis(
        &[
            ("selected/limit.rs", "pub fn selected() {}"),
            ("unrelated/broken.cpp", "void unrelated() {}"),
        ],
        &["unrelated/broken.cpp"],
        &[vec!["unrelated/broken.cpp"]],
        |analysis| {
            let report = analysis
                .capabilities
                .iter_mut()
                .find(|r| {
                    r.language == Some(Language::Rust) && r.capability == Capability::Structural
                })
                .ok_or("Rust structural capability")?;
            report.state = CapabilityState::Failed;
            report.reason = Some("Decision's applicable Rust analysis failed".into());
            Ok(())
        },
    )?;
    let mut later = canonical.latest_checkpoint.take().ok_or("Checkpoint")?;
    canonical.checkpoint_history.clear();
    canonical.context_items.clear();
    let source_id = canonical
        .sources
        .iter()
        .find(|s| matches!(s.source.payload, SourcePayload::CurrentHostUserTurn { .. }))
        .ok_or("user Source")?
        .source
        .id;
    let decision_id = DecisionId::from_bytes([100; 16]);
    let mut lifecycle = DecisionLifecycle {
        decision: Decision {
            id: decision_id,
            project_id: canonical.project.id,
            revision: 1,
            question_id: QuestionId::from_bytes([101; 16]),
            question_revision: 1,
            user_turn_source_id: source_id,
            choice: DecisionChoice::Alternative {
                alternative_key: "local".into(),
            },
            user_rationale: Some("Keep the selected boundary local".into()),
            displayed_alternatives: Vec::new(),
            displayed_recommendation: AgentRecommendation {
                alternative_key: None,
                rationale: "No recommendation".into(),
                source_basis: Vec::new(),
            },
            work_scope: DecisionWorkScope::ProjectWide,
            applicability: ApplicabilityScope::default(),
            assumptions: Vec::new(),
            revisit_triggers: Vec::new(),
            recorded_at: canonical.project.updated_at,
        },
        superseded_by: None,
        contradictions: Vec::new(),
        review_due: None,
    };
    // Exercise both declared paths and components resolved against actual entities.
    for component_scope in [false, true] {
        lifecycle.decision.applicability = if component_scope {
            ApplicabilityScope {
                components: vec!["selected/limit.rs".into()],
                ..Default::default()
            }
        } else {
            ApplicabilityScope {
                paths: vec!["selected/limit.rs".into()],
                ..Default::default()
            }
        };
        for superseded in [false, true] {
            canonical.active_decisions.clear();
            canonical.superseded_decisions.clear();
            lifecycle.superseded_by = superseded.then_some(DecisionId::from_bytes([102; 16]));
            if superseded {
                canonical.superseded_decisions.push(lifecycle.clone());
            } else {
                canonical.active_decisions.push(lifecycle.clone());
            }
            let project = |basis: &volicord_context::CanonicalReadBasis, exact: bool| {
                build_project_projection(ProjectProjectionInputs {
                    requirements: ProjectionReadRequirements::default(),
                    metadata: &[],
                    detail: ProjectionDetail {
                        decision: exact.then_some(decision_id),
                        ..Default::default()
                    },
                    selection: WorkSelector::LatestWork,
                    analysis_issues: &[],
                    canonical: basis,
                    analyses: &[&analysis],
                    applicability: volicord_inquiry::ApplicabilityQuery {
                        project_id: basis.project.id,
                        paths: Vec::new(),
                        components: Vec::new(),
                        work_contexts: Vec::new(),
                        current_assumptions: Vec::new(),
                        met_revisit_triggers: Vec::new(),
                    },
                    candidates: CandidateProjectionInput::Available(&candidates),
                    candidate_content_access: CandidateContentAccess::PolicyWithheld,
                    observed_at: basis.project.updated_at,
                    bound: ProjectionBound::default(),
                })
            };
            canonical.latest_checkpoint = None;
            canonical.checkpoint_history.clear();
            let before = project(&canonical, true)?;
            later.work_item_id = None;
            assert!(!later.applied_decisions.contains(&decision_id));
            canonical.latest_checkpoint = Some(later.clone());
            canonical.checkpoint_history.push(later.clone());
            let after = project(&canonical, true)?;
            assert_eq!(
                after
                    .selected_decision
                    .as_ref()
                    .map(|d| d.decision.decision_id),
                Some(decision_id)
            );
            assert_eq!(before.answer_capability_gaps, after.answer_capability_gaps);
            assert!(after
                .answer_capability_gaps
                .iter()
                .any(|g| g.reason == "Decision's applicable Rust analysis failed"));
            assert!(after
                .answer_capability_gaps
                .iter()
                .all(|g| g.language != Some(Language::Cpp)));
            // Reading the Decision must not widen the general current-Work topology.
            let current = project(&canonical, false)?;
            assert_eq!(after.current_work_topology, current.current_work_topology);
            assert!(current
                .current_work_topology
                .entities
                .iter()
                .all(|e| e.locator != "selected/limit.rs"));
        }
    }
    Ok(())
}

#[test]
fn materialized_repository_reading_matches_live_scope_before_and_after_bounds(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_context::{SourceFreshness, SourceId};
    use volicord_projections::{ProjectionDetail, ProjectionReadRequirements, WorkSelector};
    use volicord_repository_intelligence::{AreaId, AreaKind, Capability, CapabilityState};

    let (mut canonical, analysis, candidates) = projection_scenario_basis(
        &[
            ("work/lib.rs", "pub fn work() { helper(); } fn helper() {}"),
            ("other/large.rs", "pub fn other() { a(); b(); c(); d(); e(); f(); } fn a() {} fn b() {} fn c() {} fn d() {} fn e() {} fn f() {}"),
            ("other/query.cpp", "void query() {}"),
        ],
        &["work/lib.rs"],
        &[vec!["work/lib.rs"]],
        |analysis| {
            // Keep this cardinality fixture fully resolved; unresolved evidence has
            // separate downstream accounting and is covered by existing tests.
            for fact in &mut analysis.structural_facts {
                fact.relations.retain(|r| matches!(r.target, RelationTarget::ResolvedEntity(_)));
            }
            analysis.semantic_results.retain(|r| matches!(r.relation.target, RelationTarget::ResolvedEntity(_)));
            let mut failure = analysis.capabilities.iter()
                .find(|r| r.language == Some(Language::Rust) && r.capability == Capability::Structural)
                .ok_or("Rust capability")?.clone();
            failure.state = CapabilityState::Failed;
            failure.area = AreaId { kind: AreaKind::File, path: "other/large.rs".into() };
            failure.coverage.failed = vec![failure.area.clone()];
            failure.reason = Some("Repository-only Rust failure".into());
            analysis.capabilities.push(failure);
            Ok(())
        },
    )?;
    // An unrelated unavailable Source belongs to Repository reading, not this Work.
    let mut unavailable = canonical.sources[0].clone();
    let unavailable_id = SourceId::from_bytes([150; 16]);
    unavailable.source.id = unavailable_id;
    unavailable.freshness = SourceFreshness::Unavailable;
    unavailable.availability = Availability::Unavailable;
    unavailable.source.payload = SourcePayload::File {
        locator: "other/unavailable.rs".into(),
        snapshot: "fixture".into(),
    };
    canonical.sources.push(unavailable);
    let work_id = canonical
        .latest_checkpoint
        .as_ref()
        .ok_or("Checkpoint")?
        .work_item_id
        .ok_or("Work")?;
    let original = canonical.clone();
    let project = |selection, detail, limit| {
        build_project_projection(ProjectProjectionInputs {
            selection,
            detail,
            requirements: ProjectionReadRequirements::default(),
            metadata: &[],
            analysis_issues: &[],
            canonical: &canonical,
            analyses: &[&analysis],
            applicability: volicord_inquiry::ApplicabilityQuery {
                project_id: canonical.project.id,
                paths: Vec::new(),
                components: Vec::new(),
                work_contexts: Vec::new(),
                current_assumptions: Vec::new(),
                met_revisit_triggers: Vec::new(),
            },
            candidates: CandidateProjectionInput::Available(&candidates),
            candidate_content_access: CandidateContentAccess::PolicyWithheld,
            observed_at: canonical.project.updated_at,
            bound: ProjectionBound {
                max_items_per_section: limit,
            },
        })
    };
    // Include upstream map truncation and a smaller downstream architecture bound.
    for projection_limit in [1, 4, 64] {
        let work = project(
            WorkSelector::LatestWork,
            ProjectionDetail::default(),
            projection_limit,
        )?;
        let live = project(
            WorkSelector::Repository,
            ProjectionDetail::default(),
            projection_limit,
        )?;
        assert!(work.answer_capability_gaps.iter().all(
            |g| g.language != Some(Language::Cpp) && g.reason != "Repository-only Rust failure"
        ));
        if projection_limit == 1 {
            assert!(
                live.answer_capability_gaps.len() > live.repository_map.gaps.len(),
                "Repository answer relevance must be selected before the map gap bound"
            );
        }
        assert!(live
            .answer_capability_gaps
            .iter()
            .any(|g| g.language == Some(Language::Cpp)));
        assert!(live
            .answer_capability_gaps
            .iter()
            .any(|g| g.reason == "Repository-only Rust failure"));
        assert!(!work
            .answer_issues
            .iter()
            .any(|i| i.identity == unavailable_id.to_string()));
        assert!(live
            .answer_issues
            .iter()
            .any(|i| i.identity == unavailable_id.to_string()));
        if projection_limit == 4 {
            assert_ne!(
                work.current_work_topology.omitted_entity_count,
                live.current_work_topology.omitted_entity_count
            );
            assert_ne!(
                work.current_work_topology.omitted_relation_count,
                live.current_work_topology.omitted_relation_count
            );
        }
        let entity = work
            .current_work_topology
            .entities
            .first()
            .ok_or("Work code")?
            .identity
            .clone();
        for (selector, detail) in [
            (WorkSelector::LatestWork, ProjectionDetail::default()),
            (
                WorkSelector::ExactWork(work_id),
                ProjectionDetail::default(),
            ),
            (
                WorkSelector::ExactWork(work_id),
                ProjectionDetail {
                    entity: Some(entity.clone()),
                    ..Default::default()
                },
            ),
        ] {
            let selected = project(selector, detail, projection_limit)?;
            let selected_before = selected.clone();
            let offline = selected.for_repository_reading();
            assert_eq!(selected, selected_before);
            assert_eq!(offline.answer_capability_gaps, live.answer_capability_gaps);
            assert_eq!(offline.answer_issues, live.answer_issues);
            assert_eq!(offline.current_work_topology, live.current_work_topology);
            assert!(offline.selected_work.is_none() && offline.selected_entity.is_none());
            assert!(offline.selected_entity_relations.is_empty());
            for understanding_limit in [2, 32] {
                let bound = UnderstandingBound {
                    max_items_per_section: understanding_limit,
                };
                let actual = build_project_understanding(&offline, bound);
                let expected = build_project_understanding(&live, bound);
                assert_eq!(actual.architecture, expected.architecture);
                let omitted = |u: &volicord_projections::ProjectUnderstanding, section: &str| {
                    u.omissions
                        .iter()
                        .filter(|o| o.section == section)
                        .map(|o| o.omitted_count)
                        .sum::<usize>()
                };
                for section in [
                    "architecture.components",
                    "architecture.relationships",
                    "architecture.gaps",
                ] {
                    assert_eq!(omitted(&actual, section), omitted(&expected, section));
                }
                // Independent cardinality oracle: all fixture entities/relations are
                // resolved; repeated relation identities count once. Neither display
                // bound can erase their omissions.
                assert_eq!(
                    omitted(&actual, "architecture.components"),
                    analysis.structural_facts.len() - actual.architecture.components.len()
                );
                let relation_count = analysis
                    .structural_facts
                    .iter()
                    .flat_map(|f| f.relations.iter().map(|r| r.identity.as_str()))
                    .chain(
                        analysis
                            .semantic_results
                            .iter()
                            .map(|r| r.relation.identity.as_str()),
                    )
                    .collect::<std::collections::BTreeSet<_>>()
                    .len();
                assert_eq!(
                    omitted(&actual, "architecture.relationships"),
                    relation_count - actual.architecture.relationships.len()
                );
            }
        }
    }
    assert_eq!(canonical, original);
    Ok(())
}
