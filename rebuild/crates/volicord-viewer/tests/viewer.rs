use rusqlite::Connection;
use std::fs;
use tempfile::tempdir;
use volicord_context::{
    AgentRecommendation, ApplicabilityScope, Availability, CheckpointDraft, CheckpointKind, Clock,
    ContextItemDraft, ContextItemRole, ExplicitQuestionResponse, NonUserQuestionOutcome,
    OperationId, Principal, PrincipalKind, ProjectId, QuestionAlternative, QuestionDraft,
    QuestionMateriality, QuestionResearchState, QuestionResponseDraft, SourceDraft, SourcePayload,
    StatementProvenanceRole, Store, SystemClock, TimestampMicros, UserAcceptanceFact,
    UserAcceptanceState, UserReviewFact, UserReviewState, UserTurnSource, VerificationFact,
    VerificationState, WorkState,
};
use volicord_operations::{
    ConfirmationDecision, GuardedEffectCategory, GuardedEffectDraft, GuardedRisk, LocalOperations,
    RequestingProvenance, RuntimeLayout,
};
use volicord_projections::{build_project_understanding, UnderstandingBound};
use volicord_viewer::{ExplanationLevel, ViewerAdapter, ViewerLocale, ViewerRequest};

fn setup() -> (tempfile::TempDir, ViewerAdapter, ProjectId) {
    let temporary = tempdir().expect("temporary directory");
    let runtime = temporary.path().join("runtime");
    let operations = LocalOperations::new(RuntimeLayout::new(runtime).expect("layout"));
    let initialized = operations
        .initialize_project("Viewer Project", Some(temporary.path()))
        .expect("initialize Project");
    let project = initialized.project.id;
    (temporary, ViewerAdapter::new(operations), project)
}

#[test]
fn candidate_view_distinguishes_empty_unavailable_corrupt_and_unsupported_dependencies() {
    let (_healthy_root, healthy, healthy_project) = setup();
    let healthy_page = render_deep(&healthy, healthy_project);
    assert!(healthy_page.contains("No Session Candidates."));
    assert!(!healthy_page.contains("Candidate data omitted"));

    let (_unsupported_root, unsupported, unsupported_project) = setup();
    Connection::open(unsupported.operations().layout().candidate_store())
        .expect("open Candidate store")
        .execute(
            "UPDATE metadata SET value = '999' WHERE key = 'schema_version'",
            [],
        )
        .expect("set unsupported Candidate schema");
    assert_candidate_view_dependency(&unsupported, unsupported_project, "unsupported");

    let (_corrupt_root, corrupt, corrupt_project) = setup();
    Connection::open(corrupt.operations().layout().candidate_store())
        .expect("open Candidate store")
        .execute("DROP TABLE candidates", [])
        .expect("remove required Candidate table");
    assert_candidate_view_dependency(&corrupt, corrupt_project, "corrupt");

    let (_unavailable_root, unavailable, unavailable_project) = setup();
    let candidate_path = unavailable.operations().layout().candidate_store();
    fs::remove_file(&candidate_path).expect("remove Candidate store");
    fs::create_dir(&candidate_path).expect("replace Candidate store with unavailable path");
    assert_candidate_view_dependency(&unavailable, unavailable_project, "unavailable");
}

fn render_deep(viewer: &ViewerAdapter, project: ProjectId) -> String {
    viewer
        .render(
            &ViewerRequest {
                project_id: project,
                locale: ViewerLocale::English,
                explanation_level: ExplanationLevel::Deep,
                requested_language: "en".into(),
                guarded_request: None,
            },
            "test-request-authenticity",
        )
        .expect("render viewer")
        .html
}

fn assert_candidate_view_dependency(viewer: &ViewerAdapter, project: ProjectId, expected: &str) {
    let page = render_deep(viewer, project);
    assert!(page.contains("Candidate data omitted"), "{page}");
    assert!(page.contains(expected), "{page}");
    assert!(!page.contains("No Session Candidates."), "{page}");
    assert!(page.contains("Canonical context"), "{page}");
}

#[test]
fn reads_render_every_project_surface_without_mutating_canonical_state() {
    let (_temporary, viewer, project) = setup();
    let before = viewer
        .operations()
        .canonical_basis(project)
        .expect("basis before render");
    let repository_entries_before = repository_entries(&_temporary);
    let page = viewer
        .render(
            &ViewerRequest {
                project_id: project,
                locale: ViewerLocale::English,
                explanation_level: ExplanationLevel::Deep,
                requested_language: "en".into(),
                guarded_request: None,
            },
            "test-request-authenticity",
        )
        .expect("render viewer");
    let after = viewer
        .operations()
        .canonical_basis(project)
        .expect("basis after render");
    let repository_entries_after = repository_entries(&_temporary);

    assert_eq!(before, after);
    assert_eq!(repository_entries_before, repository_entries_after);
    assert!(page.html.starts_with("<!doctype html><html lang=\"en\">"));
    for expected in [
        "Project Understanding",
        "Project overview",
        "Repository Map",
        "Current Decisions",
        "Checkpoint timeline",
        "Candidate inspection",
        "Canonical context",
        "Privacy and provider",
        "Document preview / export",
        "Health and usable capability",
        "Narrative language",
        "fixed locale realized",
    ] {
        assert!(page.html.contains(expected), "missing {expected}");
    }
    let overview = page.html.find("id=\"project-overview\"").expect("overview");
    let understanding = page
        .html
        .find("id=\"project-understanding\"")
        .expect("Project Understanding");
    let decisions = page.html.find("id=\"decisions\"").expect("decisions");
    let health = page.html.find("id=\"health\"").expect("health");
    let checkpoints = page.html.find("id=\"checkpoints\"").expect("checkpoints");
    let repository = page.html.find("id=\"repository-map\"").expect("repository");
    assert!(
        understanding < overview
            && overview < health
            && health < decisions
            && decisions < checkpoints
            && checkpoints < repository
    );
    for empty in [
        "No current Project goal is recorded.",
        "No open Questions.",
        "No Decisions are recorded.",
        "No Checkpoints have been recorded.",
        "No Session Candidates.",
    ] {
        assert!(page.html.contains(empty), "missing empty state: {empty}");
    }
    assert!(page
        .html
        .contains("<section id=\"project-overview\" aria-labelledby="));
    assert!(page.html.contains("<ol class=\"timeline\"") || page.html.contains("No Checkpoints"));
    assert!(page.html.contains(":focus-visible"));
    assert!(page.html.contains("@media (max-width:44rem)"));
    assert!(page.html.contains("<fieldset>"));
    assert!(page.html.contains("class=\"document-preview\""));
    assert!(page.html.contains("class=\"preview-section\""));
    assert!(!page.html.contains("<pre>"));
    assert!(page.html.contains("data-statement-role=\"verified-fact\""));
    assert!(page.html.contains("data-diagram=\"architecture-topology\""));
}

#[test]
fn korean_fixed_text_and_all_explanation_levels_are_available() {
    let (_temporary, viewer, project) = setup();
    for level in [
        ExplanationLevel::Overview,
        ExplanationLevel::Working,
        ExplanationLevel::Deep,
    ] {
        let page = viewer
            .render(
                &ViewerRequest {
                    project_id: project,
                    locale: ViewerLocale::Korean,
                    explanation_level: level,
                    requested_language: "한국어".into(),
                    guarded_request: None,
                },
                "test-request-authenticity",
            )
            .expect("render Korean viewer");
        assert!(page.html.starts_with("<!doctype html><html lang=\"ko\">"));
        assert!(page.html.contains("프로젝트 개요"));
        assert!(page.html.contains("저장소 지도"));
        assert!(page.html.contains("문서 미리보기 / 내보내기"));
        assert!(page.html.contains("HTML 언어 태그"));
        assert!(page.html.contains("<dd>ko</dd>"));
        assert!(page.html.contains("사용 가능한 기능"));
        assert!(!page.html.contains("NeverEnabled"));
        assert!(!page.html.contains("Projection: <strong>Complete"));
    }
}

#[test]
fn arbitrary_generated_language_instruction_cannot_become_html_language_syntax() {
    let (_temporary, viewer, project) = setup();
    let requested_language = "fr-CA\" data-unsafe=\"<&";
    let page = viewer
        .render(
            &ViewerRequest {
                project_id: project,
                locale: ViewerLocale::English,
                explanation_level: ExplanationLevel::Deep,
                requested_language: requested_language.into(),
                guarded_request: None,
            },
            "test-request-authenticity",
        )
        .expect("render viewer");
    assert!(page.html.starts_with("<!doctype html><html lang=\"en\">"));
    assert!(page.html.contains("HTML language tag"));
    assert!(page.html.contains("<dd>en</dd>"));
    assert!(page
        .html
        .contains("fr-CA&quot; data-unsafe=&quot;&lt;&amp;"));
    assert!(!page.html.contains("lang=\"fr-CA&quot;"));
    assert!(page.html.contains("data-state=\"unavailable\""));
    assert!(page.html.contains("no active-host narrative realizer"));
    assert!(!page.html.contains("class=\"preview-section\""));
    assert!(!page.html.contains("action=\"/documents/export\""));
}

#[test]
fn project_understanding_diagrams_use_only_inspectable_relation_topology() {
    let (temporary, viewer, project) = setup();
    fs::write(
        temporary.path().join("package.json"),
        "{\"name\":\"grounded-viewer\",\"type\":\"module\"}\n",
    )
    .expect("write package fixture");
    fs::write(
        temporary.path().join("tsconfig.json"),
        "{\"compilerOptions\":{\"strict\":true,\"target\":\"ES2022\"},\"include\":[\"src\"]}\n",
    )
    .expect("write TypeScript configuration fixture");
    fs::create_dir(temporary.path().join("src")).expect("create source fixture directory");
    fs::write(
        temporary.path().join("src/service.ts"),
        "export function run(): string { return 'ok'; }\n",
    )
    .expect("write service fixture");
    fs::write(
        temporary.path().join("src/app.ts"),
        "import { run } from './service.js';\nexport function main(): string { return run(); }\n",
    )
    .expect("write application fixture");
    let mut store = Store::open(viewer.operations().layout().canonical_store()).expect("store");
    let revision = store.get_project(project).expect("Project").revision;
    let user_turn = store
        .record_source(
            OperationId::from_bytes([71; 16]),
            project,
            SourceDraft {
                expected_project_revision: revision,
                payload: SourcePayload::CurrentHostUserTurn {
                    host: "viewer-test".into(),
                    session: "session-grounded-explanation".into(),
                    turn: "Keep the application entry point in src/app.ts".into(),
                },
                actor: Principal {
                    kind: PrincipalKind::User,
                    identity: "owner".into(),
                },
                observer: None,
                availability: Availability::Available,
            },
        )
        .expect("user Source")
        .value;
    let question = store
        .create_question(
            OperationId::from_bytes([72; 16]),
            project,
            QuestionDraft {
                expected_project_revision: revision,
                prompt_basis: "Where should the application entry point remain?".into(),
                source_basis: vec![user_turn.id],
                dependencies: Vec::new(),
                alternatives: vec![QuestionAlternative {
                    key: "app-file".into(),
                    label: "Keep src/app.ts".into(),
                    consequence: "src/app.ts remains the declared entry-point scope".into(),
                }],
                recommendation: AgentRecommendation {
                    alternative_key: Some("app-file".into()),
                    rationale: "the current application flow starts there".into(),
                    source_basis: vec![user_turn.id],
                },
                trade_offs: Vec::new(),
                uncertainty: Vec::new(),
                material_scope: vec!["src/app.ts".into()],
                materiality: QuestionMateriality::Material,
                presentation_order: 1,
                why_it_matters_now: "the Viewer should connect Decisions to code".into(),
                established_facts: Vec::new(),
                assumptions: Vec::new(),
                known_limits: Vec::new(),
                what_the_answer_unlocks: vec!["grounded Decision impact".into()],
                allowed_non_choice_dispositions: NonUserQuestionOutcome::ALL.to_vec(),
                research_state: QuestionResearchState::ReadyToAsk,
            },
        )
        .expect("Question")
        .value;
    let decision = store
        .record_question_response(
            OperationId::from_bytes([73; 16]),
            project,
            QuestionResponseDraft {
                expected_project_revision: revision,
                question_id: question.id,
                question_revision: question.revision,
                user_turn_source: UserTurnSource::Existing(user_turn.id),
                displayed_alternative_keys: vec!["app-file".into()],
                displayed_recommendation_key: Some("app-file".into()),
                response: ExplicitQuestionResponse::Choice {
                    alternative_key: "app-file".into(),
                    user_rationale: Some("keep the entry point easy to find".into()),
                },
                applicability: ApplicabilityScope {
                    paths: vec!["src/app.ts".into()],
                    components: Vec::new(),
                    work_contexts: Vec::new(),
                },
                assumptions: Vec::new(),
                revisit_triggers: Vec::new(),
            },
        )
        .expect("Decision response")
        .value
        .decision
        .expect("Decision");
    drop(store);
    viewer
        .operations()
        .analyze(project, Vec::new())
        .expect("analyze diagram fixture");
    let projection = viewer
        .operations()
        .project_projection(project)
        .expect("Project projection");
    let understanding = build_project_understanding(
        &projection,
        UnderstandingBound {
            max_items_per_section: 32,
        },
    );
    assert!(!understanding.architecture.components.is_empty());
    assert!(!understanding.architecture.relationships.is_empty());
    assert!(understanding.generated_interpretations.is_empty());
    assert!(!understanding.deterministic_explanations.is_empty());
    assert!(understanding
        .deterministic_explanations
        .iter()
        .any(|item| !item.relation_basis.is_empty()));
    let canonical_before = viewer
        .operations()
        .canonical_basis(project)
        .expect("canonical basis before rendering");

    let request = ViewerRequest {
        project_id: project,
        locale: ViewerLocale::English,
        explanation_level: ExplanationLevel::Deep,
        requested_language: "en".into(),
        guarded_request: None,
    };
    let html = viewer
        .render(&request, "test-request-authenticity")
        .expect("render diagram Viewer")
        .html;
    assert!(html.contains("data-diagram=\"architecture-topology\""));
    assert!(html.contains("data-diagram=\"flow-topology\""));
    assert!(html.contains("role=\"img\" aria-labelledby="));
    assert!(html.contains("<title id=\"architecture-topology-title\""));
    assert!(html.contains("<desc id=\"architecture-topology-description\""));
    assert!(html.contains("data-statement-role=\"deterministic-derived\""));
    assert!(html.contains("data-explanation-kind=\"component\""));
    assert!(
        html.contains("data-explanation-kind=\"flow\""),
        "available deterministic explanations: {:?}",
        understanding
            .deterministic_explanations
            .iter()
            .map(|item| (item.kind, item.english.as_str(), &item.relation_basis))
            .collect::<Vec<_>>()
    );
    assert!(html.contains("data-explanation-kind=\"decision-impact\""));
    assert!(html.contains("Inspect evidence basis"));
    assert!(html.contains(&decision.id.to_string()));
    assert!(html.contains("not proof of implementation"));
    assert!(!html.contains("class=\"interpretation-layer\""));
    for explanation in &understanding.deterministic_explanations {
        assert!(html.contains(&format!("data-explanation-id=\"{}\"", explanation.identity)));
        for relation in &explanation.relation_basis {
            assert!(html.contains(relation));
        }
    }
    for relation in &understanding.architecture.relationships {
        if relation.target_entity.is_some() {
            assert!(
                html.contains(&format!("data-relation-id=\"{}\"", relation.identity)),
                "missing inspectable relation {}",
                relation.identity
            );
        }
    }
    for node in &understanding.architecture.components {
        assert!(
            html.contains(&format!("data-entity-id=\"{}\"", node.identity)),
            "missing inspectable entity {}",
            node.identity
        );
    }
    for forbidden in ["<script", "cdn", "mermaid", "graphviz", " src=\""] {
        assert!(!html.contains(forbidden), "found {forbidden}");
    }
    let snapshot = viewer
        .render_snapshot(&request, TimestampMicros::from_unix_micros(777_000))
        .expect("render grounded diagram snapshot")
        .html;
    assert!(snapshot.contains("data-viewer-mode=\"snapshot\""));
    assert!(snapshot.contains("data-diagram=\"architecture-topology\""));
    assert!(snapshot.contains("role=\"img\" aria-labelledby="));
    assert!(!snapshot.contains("<script"));
    assert!(!snapshot.contains(" href="));
    assert!(!snapshot.contains(" src="));
    assert!(snapshot.contains("data-statement-role=\"deterministic-derived\""));
    let korean = viewer
        .render(
            &ViewerRequest {
                project_id: project,
                locale: ViewerLocale::Korean,
                explanation_level: ExplanationLevel::Deep,
                requested_language: "한국어".into(),
                guarded_request: None,
            },
            "test-request-authenticity",
        )
        .expect("render Korean grounded explanation Viewer")
        .html;
    assert!(korean.contains("결정론적 설명"));
    assert!(korean.contains("근거 확인"));
    assert!(korean.contains("target은 resolve되지 않았습니다"));
    let canonical_after = viewer
        .operations()
        .canonical_basis(project)
        .expect("canonical basis after rendering");
    assert_eq!(canonical_before, canonical_after);
}

#[test]
fn memory_targets_and_checkpoints_are_human_identifiable_and_detailed() {
    let (_temporary, viewer, project) = setup();
    let mut store = Store::open(viewer.operations().layout().canonical_store()).expect("store");
    let revision = store.get_project(project).expect("Project").revision;
    let user_turn = store
        .record_source(
            OperationId::from_bytes([31; 16]),
            project,
            SourceDraft {
                expected_project_revision: revision,
                payload: SourcePayload::CurrentHostUserTurn {
                    host: "viewer-test".into(),
                    session: "session-operator".into(),
                    turn: "Keep mutation targets readable".into(),
                },
                actor: Principal {
                    kind: PrincipalKind::User,
                    identity: "owner".into(),
                },
                observer: None,
                availability: Availability::Available,
            },
        )
        .expect("user Source")
        .value;
    let context = store
        .record_context_item(
            OperationId::from_bytes([32; 16]),
            project,
            ContextItemDraft {
                expected_project_revision: revision,
                role: ContextItemRole::Goal,
                statement: "Keep mutation targets readable".into(),
                provenance_role: StatementProvenanceRole::UserStatement,
                author: Principal {
                    kind: PrincipalKind::User,
                    identity: "owner".into(),
                },
                source_basis: vec![user_turn.id],
                applicability: ApplicabilityScope::default(),
            },
        )
        .expect("Context Item")
        .value;
    let question = store
        .create_question(
            OperationId::from_bytes([35; 16]),
            project,
            QuestionDraft {
                expected_project_revision: revision,
                prompt_basis: "How should the Viewer present current work?".into(),
                source_basis: vec![user_turn.id],
                dependencies: Vec::new(),
                alternatives: vec![QuestionAlternative {
                    key: "operator-readable".into(),
                    label: "Operator-readable cockpit".into(),
                    consequence: "current work remains primary".into(),
                }],
                recommendation: AgentRecommendation {
                    alternative_key: Some("operator-readable".into()),
                    rationale: "the operator needs a resumable reading path".into(),
                    source_basis: vec![user_turn.id],
                },
                trade_offs: Vec::new(),
                uncertainty: Vec::new(),
                material_scope: vec!["viewer".into()],
                materiality: QuestionMateriality::Material,
                presentation_order: 1,
                why_it_matters_now: "the current Viewer is a raw console".into(),
                established_facts: Vec::new(),
                assumptions: Vec::new(),
                known_limits: Vec::new(),
                what_the_answer_unlocks: vec!["readable presentation".into()],
                allowed_non_choice_dispositions: NonUserQuestionOutcome::ALL.to_vec(),
                research_state: QuestionResearchState::ReadyToAsk,
            },
        )
        .expect("Question")
        .value;
    let decision = store
        .record_question_response(
            OperationId::from_bytes([36; 16]),
            project,
            QuestionResponseDraft {
                expected_project_revision: revision,
                question_id: question.id,
                question_revision: question.revision,
                user_turn_source: UserTurnSource::Existing(user_turn.id),
                displayed_alternative_keys: vec!["operator-readable".into()],
                displayed_recommendation_key: Some("operator-readable".into()),
                response: ExplicitQuestionResponse::Choice {
                    alternative_key: "operator-readable".into(),
                    user_rationale: Some("resume state should be primary".into()),
                },
                applicability: ApplicabilityScope::default(),
                assumptions: Vec::new(),
                revisit_triggers: Vec::new(),
            },
        )
        .expect("Decision response")
        .value
        .decision
        .expect("Decision");
    let command = store
        .record_source(
            OperationId::from_bytes([33; 16]),
            project,
            SourceDraft {
                expected_project_revision: revision,
                payload: SourcePayload::CommandExecution {
                    command_label: "cargo test -p volicord-viewer".into(),
                    outcome: volicord_context::CommandOutcome {
                        exit_code: Some(0),
                        termination: volicord_context::CommandTermination::Exited,
                    },
                },
                actor: Principal {
                    kind: PrincipalKind::Command,
                    identity: "cargo".into(),
                },
                observer: None,
                availability: Availability::Available,
            },
        )
        .expect("command Source")
        .value;
    store
        .record_checkpoint(
            OperationId::from_bytes([34; 16]),
            project,
            CheckpointDraft {
                expected_project_revision: revision,
                kind: CheckpointKind::Completion,
                goal: "Make the Viewer operator-readable".into(),
                work_state: WorkState::Completed,
                state_change: Some("structured cockpit rendered".into()),
                source_basis: vec![user_turn.id, command.id],
                changed_source_basis: vec![command.id],
                changed_paths: vec!["rebuild/crates/volicord-viewer/src/render.rs".into()],
                applied_decisions: Vec::new(),
                verification: vec![VerificationFact {
                    state: VerificationState::Passed,
                    source_id: Some(command.id),
                    outcome: Some("viewer tests passed".into()),
                }],
                user_review: UserReviewFact {
                    state: UserReviewState::Pending,
                    source_id: None,
                },
                user_acceptance: UserAcceptanceFact {
                    state: UserAcceptanceState::NotRequested,
                    source_id: None,
                },
                known_limits: vec!["manual zoom review remains".into()],
                non_goals: Vec::new(),
                open_questions: Vec::new(),
                next_step: "No further work is planned for this goal".into(),
                handoff_to: None,
            },
        )
        .expect("Checkpoint");
    drop(store);

    let page = viewer
        .render(
            &ViewerRequest {
                project_id: project,
                locale: ViewerLocale::English,
                explanation_level: ExplanationLevel::Deep,
                requested_language: "en".into(),
                guarded_request: None,
            },
            "test-request-authenticity",
        )
        .expect("render rich Viewer");
    assert!(page
        .html
        .contains("<summary><strong>Context Item</strong>: Keep mutation targets readable"));
    assert!(page.html.contains(&context.id.to_string()));
    assert!(page
        .html
        .contains("<summary><strong>Decision</strong>: Alternative: operator-readable"));
    assert!(page.html.contains(&decision.id.to_string()));
    assert!(page.html.contains("action=\"/memory/decision/supersede\""));
    assert!(page.html.contains("cargo test -p volicord-viewer"));
    assert!(page.html.contains("viewer tests passed"));
    assert!(page.html.contains("structured cockpit rendered"));
    assert!(page
        .html
        .contains("rebuild/crates/volicord-viewer/src/render.rs"));
    assert!(page.html.contains("manual zoom review remains"));
    assert!(page
        .html
        .contains("No further work is planned for this goal"));
    assert!(page.html.contains("<time datetime=\"unix-micros:"));
    assert!(page.html.contains("User review</dt><dd>pending"));
    assert!(page.html.contains("User acceptance</dt><dd>not requested"));

    let working = viewer
        .render(
            &ViewerRequest {
                project_id: project,
                locale: ViewerLocale::English,
                explanation_level: ExplanationLevel::Working,
                requested_language: "en".into(),
                guarded_request: None,
            },
            "test-request-authenticity",
        )
        .expect("render completed working Viewer")
        .html;
    let overview = section_html(&working, "project-overview");
    for expected in [
        "Keep mutation targets readable",
        "completed",
        "passed",
        "Alternative: operator-readable",
        "No open Questions.",
        "No further work is planned for this goal",
    ] {
        assert!(
            overview.contains(expected),
            "missing {expected}: {overview}"
        );
    }
    let work = overview.find("Resume state").expect("work summary");
    let decision_position = overview.find("Latest Decision").expect("Decision summary");
    let questions = overview.find("Open Questions").expect("Question summary");
    let next = overview.find("Next step").expect("next-step summary");
    assert!(work < decision_position && decision_position < questions && questions < next);
    assert!(!overview.contains(&project.to_string()));
    assert!(!overview.contains(&context.id.to_string()));
    assert!(!overview.contains(&decision.id.to_string()));
    assert!(!working.contains("id=\"candidates\""));
    assert!(!working.contains("id=\"canonical-context\""));
}

#[test]
fn degraded_working_view_keeps_material_gap_visible_before_audit_detail() {
    let (temporary, viewer, project) = setup();
    fs::write(
        temporary.path().join("main.go"),
        "package main\nfunc main() {}\n",
    )
    .expect("write unsupported-language fixture");
    viewer
        .operations()
        .analyze(project, Vec::new())
        .expect("analyze degraded fixture");

    let page = viewer
        .render(
            &ViewerRequest {
                project_id: project,
                locale: ViewerLocale::English,
                explanation_level: ExplanationLevel::Working,
                requested_language: "en".into(),
                guarded_request: None,
            },
            "test-request-authenticity",
        )
        .expect("render degraded working Viewer")
        .html;
    let overview = page.find("id=\"project-overview\"").expect("overview");
    let health = page.find("id=\"health\"").expect("health");
    let decisions = page.find("id=\"decisions\"").expect("decisions");
    let repository = page.find("id=\"repository-map\"").expect("repository");
    assert!(overview < health && health < decisions && decisions < repository);
    assert!(page.contains("Affected capability and scope"), "{page}");
    assert!(
        page.contains("unavailable") || page.contains("partial") || page.contains("unsupported"),
        "{page}"
    );
    let capability_summary = page
        .find("Capability coverage detail")
        .expect("progressively disclosed capability detail");
    assert!(health < capability_summary);
    assert!(!page.contains("id=\"candidates\""));
    assert!(!page.contains("id=\"canonical-context\""));
}

#[test]
fn static_snapshot_is_deterministic_self_contained_and_read_only() {
    let (temporary, viewer, project) = setup();
    let request = ViewerRequest {
        project_id: project,
        locale: ViewerLocale::English,
        explanation_level: ExplanationLevel::Working,
        requested_language: "en".into(),
        guarded_request: None,
    };
    let generated_at = TimestampMicros::from_unix_micros(123_456_789);
    let canonical_before = viewer
        .operations()
        .canonical_basis(project)
        .expect("canonical basis before snapshot");
    let repository_before = repository_entries(&temporary);
    let first = viewer
        .render_snapshot(&request, generated_at)
        .expect("render static snapshot");
    let second = viewer
        .render_snapshot(&request, generated_at)
        .expect("render deterministic static snapshot");
    assert_eq!(first, second);
    assert_eq!(
        viewer
            .operations()
            .canonical_basis(project)
            .expect("canonical basis after snapshot"),
        canonical_before
    );
    assert_eq!(repository_entries(&temporary), repository_before);
    assert!(first.html.contains("data-viewer-mode=\"snapshot\""));
    assert!(first.html.contains("Read-only static snapshot"));
    assert!(first.html.contains("Snapshot basis"));
    assert!(first.html.contains("123456789"));
    assert!(first.html.contains(&project.to_string()));
    assert!(first.html.contains("Generated document previews"));
    for forbidden in [
        "<form",
        "request_authenticity",
        "action=\"/",
        "<script",
        " href=",
        " src=",
        "/memory/",
        "/guarded/",
        "/documents/export",
    ] {
        assert!(!first.html.contains(forbidden), "found {forbidden}");
    }
    let overview = first
        .html
        .find("id=\"project-overview\"")
        .expect("overview");
    let health = first.html.find("id=\"health\"").expect("health");
    let decisions = first.html.find("id=\"decisions\"").expect("decisions");
    let checkpoints = first.html.find("id=\"checkpoints\"").expect("checkpoints");
    assert!(overview < health && health < decisions && decisions < checkpoints);
}

#[test]
fn degraded_static_snapshot_preserves_capability_honesty() {
    let (temporary, viewer, project) = setup();
    fs::write(
        temporary.path().join("snapshot.go"),
        "package snapshot\nfunc Gap() {}\n",
    )
    .expect("write degraded snapshot fixture");
    viewer
        .operations()
        .analyze(project, Vec::new())
        .expect("analyze degraded snapshot fixture");
    let html = viewer
        .render_snapshot(
            &ViewerRequest {
                project_id: project,
                locale: ViewerLocale::English,
                explanation_level: ExplanationLevel::Working,
                requested_language: "en".into(),
                guarded_request: None,
            },
            TimestampMicros::from_unix_micros(987_654_321),
        )
        .expect("render degraded snapshot")
        .html;
    assert!(html.contains("Affected capability and scope"));
    assert!(html.contains("Capability coverage detail"));
    assert!(
        html.contains("unavailable") || html.contains("partial") || html.contains("unsupported")
    );
    assert!(html.contains("Usable remainder"));
    assert!(!html.contains("<form"));
}

#[test]
fn representative_large_repository_page_is_deterministically_bounded() {
    let (temporary, viewer, project) = setup();
    for index in 0..192 {
        fs::write(
            temporary.path().join(format!("module_{index:03}.py")),
            format!("def function_{index:03}():\n    return {index}\n"),
        )
        .expect("large fixture file");
    }
    viewer
        .operations()
        .analyze(project, Vec::new())
        .expect("analyze representative large fixture");
    let request = ViewerRequest {
        project_id: project,
        locale: ViewerLocale::English,
        explanation_level: ExplanationLevel::Deep,
        requested_language: "en".into(),
        guarded_request: None,
    };
    let first = viewer
        .render(&request, "test-request-authenticity")
        .expect("first large render");
    let second = viewer
        .render(&request, "test-request-authenticity")
        .expect("second large render");
    assert_eq!(first.html.len(), second.html.len());
    assert!(first
        .html
        .contains("data-bound-scope=\"repository entities\""));
    assert!(first.html.contains("omitted by deterministic bounds"));
    assert!(first.html.contains("Affected capability and scope"));
    assert!(first.html.contains("Raw diagnostic evidence"));
    assert!(first
        .html
        .find("Affected capability and scope")
        .is_some_and(|summary| first
            .html
            .find("Raw diagnostic evidence")
            .is_some_and(|audit| summary < audit)));
    assert!(first
        .html
        .contains("Opaque identities, relations, and gap evidence"));
    assert!(first.html.matches("class=\"document-preview\"").count() == 4);
    assert!(!first.html.contains("<pre>"));
    // This fixture-specific regression detects accidental unbounded rendering;
    // it is not a universal product or hardware ceiling.
    assert!(
        first.html.len() < 180_000,
        "large page was {} bytes",
        first.html.len()
    );
}

#[test]
fn guarded_fallback_preserves_exact_request_revision_and_source_linkage() {
    let (_temporary, viewer, project) = setup();
    let now = SystemClock.now().expect("clock");
    let request = viewer
        .operations()
        .create_guarded_request(GuardedEffectDraft {
            project_id: project,
            exact_action: "publish release".into(),
            target: "registry/example".into(),
            expected_effect: "public release".into(),
            risk: GuardedRisk {
                category: GuardedEffectCategory::ExternalDeploymentOrPublicPublication,
                concrete_consequence: "public artifact".into(),
            },
            scope: vec!["release:example".into()],
            expires_at: TimestampMicros::from_unix_micros(now.as_unix_micros() + 60_000_000),
            requesting_provenance: RequestingProvenance {
                actor: Principal {
                    kind: PrincipalKind::Agent,
                    identity: "test-agent".into(),
                },
                host: Some("codex".into()),
                session: Some("session-1".into()),
                basis: vec!["test".into()],
            },
        })
        .expect("create Guarded request");
    let page = viewer
        .render(
            &ViewerRequest {
                project_id: project,
                locale: ViewerLocale::English,
                explanation_level: ExplanationLevel::Working,
                requested_language: "en".into(),
                guarded_request: Some(request.confirmation_request_identity),
            },
            "test-request-authenticity",
        )
        .expect("render Guarded fallback");
    assert!(page
        .html
        .contains(&request.confirmation_request_identity.to_string()));
    assert!(page.html.contains(&request.effect_fingerprint));

    let response = viewer
        .confirm_guarded(
            request.confirmation_request_identity,
            request.request_revision,
            &request.effect_fingerprint,
            ConfirmationDecision::Confirmed,
            "viewer-session".into(),
            "I confirm this exact release".into(),
        )
        .expect("record viewer confirmation");
    assert_eq!(
        response.confirmation_request_identity,
        request.confirmation_request_identity
    );
    assert_eq!(response.request_revision, request.request_revision);
    assert_eq!(response.effect_fingerprint, request.effect_fingerprint);
    let canonical = viewer
        .operations()
        .canonical_basis(project)
        .expect("canonical basis");
    assert!(canonical
        .sources
        .iter()
        .any(|source| source.source.id == response.user_response_source_id));
}

fn repository_entries(temporary: &tempfile::TempDir) -> Vec<String> {
    let mut entries = std::fs::read_dir(temporary.path())
        .expect("read fixture repository")
        .map(|entry| {
            entry
                .expect("repository entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    entries.sort();
    entries
}

fn section_html<'a>(page: &'a str, identity: &str) -> &'a str {
    let start = page
        .find(&format!("<section id=\"{identity}\""))
        .expect("section start");
    let remainder = &page[start..];
    let end = remainder.find("</section>").expect("section end");
    &remainder[..end]
}
