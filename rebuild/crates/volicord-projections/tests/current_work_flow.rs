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
    analyze_repository_semantics, CanonicalGrounding, InventoryRequest, Language,
    SemanticAnalysisRequest, StructuralAnalysisRequest, StructuralRelationKind,
};

fn operation(value: u8) -> OperationId {
    OperationId::from_bytes([value; 16])
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
                    turn: format!("Continue work in {changed_path}"),
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
    store.record_context_item(
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
                paths: vec![changed_path.into()],
                components: Vec::new(),
                work_contexts: vec!["viewer-current-work".into()],
            },
        },
    )?;
    store.record_checkpoint(
        operation(5),
        project.id,
        CheckpointDraft {
            expected_project_revision: project.revision,
            kind: CheckpointKind::Handoff,
            goal: "Keep the current work flow inspectable".into(),
            work_state: WorkState::Paused,
            state_change: Some("Repository Intelligence analysis is available".into()),
            source_basis: vec![repository_source.id, user_turn.id],
            changed_source_basis: vec![repository_source.id],
            changed_paths: vec![changed_path.into()],
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
    let canonical = store.read_canonical_basis(
        project.id,
        CanonicalReadOptions {
            include_checkpoint_history: true,
        },
    )?;
    let grounding = CanonicalGrounding::from_read_basis(&canonical)?;
    let (_, analysis) = analyze_repository_semantics(SemanticAnalysisRequest::new(
        StructuralAnalysisRequest::new(InventoryRequest::new(
            &repository,
            &grounding,
            repository_source.id,
            1,
        )?),
    ))?;
    let candidates = CandidateStore::open(temporary.path().join("candidates.sqlite3"))?
        .read_basis(project.id)?;
    let projection = build_project_projection(ProjectProjectionInputs {
        analysis_issues: &[],
        canonical: &canonical,
        analyses: &[&analysis],
        applicability: volicord_inquiry::ApplicabilityQuery {
            project_id: project.id,
            paths: vec![changed_path.into()],
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
    });
    Ok((projection, analysis))
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
