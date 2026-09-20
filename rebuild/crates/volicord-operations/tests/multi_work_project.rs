use rusqlite::Connection;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;
use volicord_context::{
    AgentRecommendation, ApplicabilityScope, Availability, CanonicalReadOptions, CheckpointDraft,
    CheckpointKind, ContextItemDraft, ContextItemRole, DecisionWorkScope, ExplicitQuestionResponse,
    NonUserQuestionOutcome, OperationId, Principal, PrincipalKind, ProjectId, QuestionAlternative,
    QuestionDraft, QuestionMateriality, QuestionResearchState, QuestionResponseDraft, SourceDraft,
    SourcePayload, StatementProvenanceRole, Store, TimestampMicros, UserAcceptanceFact,
    UserAcceptanceState, UserReviewFact, UserReviewState, UserTurnSource, VerificationFact,
    VerificationState, WorkState,
};
use volicord_operations::{run_cli, CliExit, LocalOperations, RuntimeLayout};
use volicord_projections::{
    build_project_understanding, generate_documents, DocumentRequest, FixedLocale,
    GeneratorIdentity, UnderstandingBound, UnderstandingWorkState,
};

fn operation(value: u8) -> OperationId {
    OperationId::from_bytes([value; 16])
}

fn principal(kind: PrincipalKind, identity: &str) -> Principal {
    Principal {
        kind,
        identity: identity.to_owned(),
    }
}

fn question_draft(
    project_revision: u64,
    source: volicord_context::SourceId,
    prompt: &str,
) -> QuestionDraft {
    QuestionDraft {
        expected_project_revision: project_revision,
        prompt_basis: prompt.to_owned(),
        source_basis: vec![source],
        dependencies: Vec::new(),
        alternatives: vec![
            QuestionAlternative {
                key: "direct".into(),
                label: "Direct".into(),
                consequence: "Keep the work bounded".into(),
            },
            QuestionAlternative {
                key: "defer".into(),
                label: "Defer".into(),
                consequence: "Leave the work open".into(),
            },
        ],
        recommendation: AgentRecommendation {
            alternative_key: Some("direct".into()),
            rationale: "The bounded work has an exact source basis".into(),
            source_basis: vec![source],
        },
        trade_offs: Vec::new(),
        uncertainty: Vec::new(),
        material_scope: vec!["multi-work fixture".into()],
        materiality: QuestionMateriality::Material,
        presentation_order: 1,
        why_it_matters_now: "The Work Item needs an explicit Decision".into(),
        established_facts: Vec::new(),
        assumptions: Vec::new(),
        known_limits: Vec::new(),
        what_the_answer_unlocks: vec!["a grouped Checkpoint".into()],
        allowed_non_choice_dispositions: NonUserQuestionOutcome::ALL.to_vec(),
        research_state: QuestionResearchState::ReadyToAsk,
    }
}

fn decision_with_scope(
    store: &mut Store,
    project_id: ProjectId,
    project_revision: u64,
    source: volicord_context::SourceId,
    work_scope: DecisionWorkScope,
    operation_base: u8,
) -> Result<volicord_context::Decision, Box<dyn std::error::Error>> {
    let question = store
        .create_question(
            operation(operation_base),
            project_id,
            question_draft(
                project_revision,
                source,
                "Which bounded implementation should this work use?",
            ),
        )?
        .value;
    Ok(store
        .record_question_response(
            operation(operation_base + 1),
            project_id,
            QuestionResponseDraft {
                expected_project_revision: project_revision,
                question_id: question.id,
                question_revision: question.revision,
                user_turn_source: UserTurnSource::Existing(source),
                displayed_alternative_keys: vec!["direct".into(), "defer".into()],
                displayed_recommendation_key: Some("direct".into()),
                response: ExplicitQuestionResponse::Choice {
                    alternative_key: "direct".into(),
                    user_rationale: Some("Keep this exact work effort bounded".into()),
                },
                work_scope,
                applicability: ApplicabilityScope {
                    paths: Vec::new(),
                    components: vec![format!("component-{operation_base}")],
                    work_contexts: Vec::new(),
                },
                assumptions: Vec::new(),
                revisit_triggers: Vec::new(),
            },
        )?
        .value
        .decision
        .ok_or("Decision was not recorded")?)
}

#[allow(clippy::too_many_arguments)]
fn checkpoint(
    store: &mut Store,
    project_id: ProjectId,
    project_revision: u64,
    work_item_id: volicord_context::ContextItemId,
    goal: &str,
    source: volicord_context::SourceId,
    changed_source: volicord_context::SourceId,
    verification_source: Option<volicord_context::SourceId>,
    decision: volicord_context::DecisionId,
    operation_id: u8,
    kind: CheckpointKind,
    state: WorkState,
    path: &str,
    next_step: &str,
) -> Result<volicord_context::Checkpoint, Box<dyn std::error::Error>> {
    Ok(store
        .record_checkpoint(
            operation(operation_id),
            project_id,
            CheckpointDraft {
                expected_project_revision: project_revision,
                work_item_id: Some(work_item_id),
                kind,
                goal: goal.to_owned(),
                work_state: state,
                state_change: Some(format!("{goal} changed")),
                source_basis: vec![source],
                changed_source_basis: vec![changed_source],
                changed_paths: vec![path.to_owned()],
                applied_decisions: vec![decision],
                verification: vec![VerificationFact {
                    state: if verification_source.is_some() {
                        VerificationState::Passed
                    } else {
                        VerificationState::NotRun
                    },
                    source_id: verification_source,
                    outcome: verification_source.map(|_| "focused verification passed".into()),
                }],
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
                next_step: next_step.to_owned(),
                handoff_to: (kind == CheckpointKind::Handoff).then(|| "next-agent".into()),
            },
        )?
        .value)
}

#[test]
fn multi_work_identity_survives_restart_portability_and_read_consumers(
) -> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempdir()?;
    let runtime = temporary.path().join("runtime");
    let repository = temporary.path().join("repository");
    fs::create_dir_all(repository.join("src"))?;
    fs::write(repository.join("src/lib.rs"), "pub fn fixture() {}\n")?;

    let (exit, init, error) = project_cli(
        &runtime,
        &repository,
        vec!["--json", "init", "Multi Work Fixture"],
    );
    assert_eq!(exit, CliExit::SUCCESS, "{error}");
    let _: Value = serde_json::from_str(&init)?;
    let (exit, _, error) = project_cli(&runtime, &repository, vec!["--json", "analyze"]);
    assert_eq!(exit, CliExit::SUCCESS, "{error}");

    let layout = RuntimeLayout::new(runtime.clone())?;
    let project_bytes: Vec<u8> = Connection::open(layout.canonical_store())?.query_row(
        "SELECT id FROM projects LIMIT 1",
        [],
        |row| row.get(0),
    )?;
    let project_id = ProjectId::from_slice(&project_bytes)?;
    let mut store = Store::open(layout.canonical_store())?;
    let project = store.get_project(project_id)?;
    let user = store
        .record_source(
            operation(101),
            project_id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::CurrentHostUserTurn {
                    host: "test".into(),
                    session: "multi-work".into(),
                    turn: "Keep the project understandable; complete Alpha, continue Beta, and leave Gamma open".into(),
                },
                actor: principal(PrincipalKind::User, "owner"),
                observer: Some(principal(PrincipalKind::Agent, "test-agent")),
                availability: Availability::Available,
            },
        )?
        .value;
    let repository_source = store
        .record_source(
            operation(102),
            project_id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::File {
                    locator: "src/lib.rs".into(),
                    snapshot: "multi-work-fixture".into(),
                },
                actor: principal(PrincipalKind::Repository, "fixture"),
                observer: None,
                availability: Availability::Available,
            },
        )?
        .value;
    let command = store
        .record_source(
            operation(103),
            project_id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::CommandExecution {
                    command_label: "focused multi-work verification".into(),
                    invocation_fingerprint: format!("sha256:{}", "1".repeat(64)),
                    outcome: volicord_context::CommandOutcome {
                        exit_code: Some(0),
                        termination: volicord_context::CommandTermination::Exited,
                    },
                },
                actor: principal(PrincipalKind::Command, "test-runner"),
                observer: Some(principal(PrincipalKind::Agent, "test-agent")),
                availability: Availability::Available,
            },
        )?
        .value;

    let record_context = |store: &mut Store,
                          operation_id,
                          role,
                          statement: &str|
     -> Result<_, volicord_context::Error> {
        store.record_context_item(
            operation(operation_id),
            project_id,
            ContextItemDraft {
                expected_project_revision: project.revision,
                role,
                statement: statement.to_owned(),
                provenance_role: StatementProvenanceRole::UserStatement,
                author: principal(PrincipalKind::User, "owner"),
                source_basis: vec![user.id],
                applicability: ApplicabilityScope::default(),
            },
        )
    };
    let purpose = record_context(
        &mut store,
        104,
        ContextItemRole::ProjectPurpose,
        "Help maintainers understand and resume the project",
    )?
    .value;
    let alpha = record_context(&mut store, 105, ContextItemRole::Goal, "Complete Alpha")?.value;
    let beta = record_context(&mut store, 106, ContextItemRole::Goal, "Continue Beta")?.value;
    let gamma = record_context(&mut store, 107, ContextItemRole::Goal, "Explore Gamma")?.value;
    let alpha_decision = decision_with_scope(
        &mut store,
        project_id,
        project.revision,
        user.id,
        DecisionWorkScope::WorkItem(alpha.id),
        108,
    )?;
    let beta_decision = decision_with_scope(
        &mut store,
        project_id,
        project.revision,
        user.id,
        DecisionWorkScope::WorkItem(beta.id),
        110,
    )?;
    let alpha_pause = checkpoint(
        &mut store,
        project_id,
        project.revision,
        alpha.id,
        &alpha.statement,
        user.id,
        repository_source.id,
        None,
        alpha_decision.id,
        112,
        CheckpointKind::Pause,
        WorkState::Paused,
        "src/alpha.rs",
        "finish Alpha",
    )?;
    let alpha_complete = checkpoint(
        &mut store,
        project_id,
        project.revision,
        alpha.id,
        &alpha.statement,
        user.id,
        repository_source.id,
        Some(command.id),
        alpha_decision.id,
        113,
        CheckpointKind::Completion,
        WorkState::Completed,
        "src/alpha.rs",
        "monitor Alpha",
    )?;
    let beta_checkpoint = checkpoint(
        &mut store,
        project_id,
        project.revision,
        beta.id,
        &beta.statement,
        user.id,
        repository_source.id,
        None,
        beta_decision.id,
        114,
        CheckpointKind::Handoff,
        WorkState::InProgress,
        "src/beta.rs",
        "continue Beta",
    )?;
    let unresolved_decision = decision_with_scope(
        &mut store,
        project_id,
        project.revision,
        user.id,
        DecisionWorkScope::Unresolved,
        116,
    )?;

    let bundle = temporary.path().join("multi-work.json");
    store.export_bundle(project_id, &bundle)?;
    let imported_path = temporary.path().join("imported.sqlite3");
    let mut imported = Store::open(&imported_path)?;
    imported.import_bundle(operation(118), &bundle)?;
    let imported_basis =
        imported.read_canonical_basis(project_id, CanonicalReadOptions::default())?;
    assert_eq!(
        imported_basis
            .context_items
            .iter()
            .filter(|item| item.role == ContextItemRole::ProjectPurpose)
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        vec![purpose.id]
    );
    assert_eq!(
        imported
            .get_checkpoint(project_id, alpha_pause.id)?
            .work_item_id,
        Some(alpha.id)
    );
    assert_eq!(
        imported
            .get_checkpoint(project_id, alpha_complete.id)?
            .work_item_id,
        Some(alpha.id)
    );
    assert_eq!(
        imported
            .get_checkpoint(project_id, beta_checkpoint.id)?
            .work_item_id,
        Some(beta.id)
    );
    assert!(imported_basis.active_decisions.iter().any(|decision| {
        decision.decision.id == alpha_decision.id
            && decision.decision.work_scope == DecisionWorkScope::WorkItem(alpha.id)
    }));
    assert!(imported_basis.active_decisions.iter().any(|decision| {
        decision.decision.id == unresolved_decision.id
            && decision.decision.work_scope == DecisionWorkScope::Unresolved
    }));
    drop(imported);
    drop(store);

    let restarted = LocalOperations::new(RuntimeLayout::new(runtime.clone())?);
    let projection = restarted.project_projection(project_id)?;
    let understanding = build_project_understanding(
        &projection,
        UnderstandingBound {
            max_items_per_section: 32,
        },
    );
    assert_eq!(understanding.project_purpose[0].identity, purpose.id);
    assert_eq!(understanding.work_history.len(), 3);
    let alpha_work = understanding
        .work_history
        .iter()
        .find(|work| work.work_item_id == alpha.id)
        .ok_or("Alpha Work Item missing")?;
    assert_eq!(
        alpha_work.checkpoint_ids,
        vec![alpha_pause.id, alpha_complete.id]
    );
    assert_eq!(alpha_work.decision_ids, vec![alpha_decision.id]);
    assert_eq!(alpha_work.state, UnderstandingWorkState::Completed);
    assert!(alpha_work
        .changed_components
        .contains(&"component-108".into()));
    assert_eq!(understanding.current_work[0].work_item_id, beta.id);
    assert_eq!(understanding.completed_work[0].work_item_id, alpha.id);
    assert_eq!(understanding.remaining_work[0].work_item_id, gamma.id);
    assert!(understanding.unresolved_work_grouping.iter().any(|gap| {
        gap.record_kind == "decision" && gap.identity == unresolved_decision.id.to_string()
    }));

    let documents = generate_documents(
        &projection,
        &DocumentRequest {
            requested_language: "en".into(),
            fixed_locale: FixedLocale::English,
            generated_at: TimestampMicros::from_unix_micros(999_000),
            generator: GeneratorIdentity {
                generator: "multi-work-test".into(),
                agent: None,
                model: None,
            },
            requested_destinations: Vec::new(),
        },
    )?;
    assert!(documents
        .project_architecture_guide
        .markdown
        .content
        .contains("Help maintainers understand and resume the project"));
    assert!(documents
        .implementation_plan
        .markdown
        .content
        .contains("Complete Alpha"));
    assert!(documents
        .handoff_resume
        .markdown
        .content
        .contains("Continue Beta"));

    let (exit, output, error) = project_cli(&runtime, &repository, vec!["--json", "status"]);
    assert_eq!(exit, CliExit::SUCCESS, "{error}");
    let status: Value = serde_json::from_str(&output)?;
    assert_eq!(status["project_purpose"][0]["statement"], purpose.statement);
    assert_eq!(status["work_history"].as_array().map(Vec::len), Some(3));
    assert_eq!(
        status["current_work"][0]["work_item_id"],
        beta.id.to_string()
    );
    assert_eq!(
        status["completed_work"][0]["checkpoint_ids"],
        serde_json::json!([alpha_pause.id.to_string(), alpha_complete.id.to_string()])
    );
    assert_eq!(
        status["remaining_work"][0]["work_item_id"],
        gamma.id.to_string()
    );
    assert_eq!(
        status["unresolved_work_grouping"][0]["identity"],
        unresolved_decision.id.to_string()
    );
    Ok(())
}

fn project_cli<'a>(
    runtime: &'a std::path::Path,
    repository: &'a std::path::Path,
    args: Vec<&'a str>,
) -> (CliExit, String, String) {
    let mut command = vec![
        "--runtime",
        runtime.to_str().expect("runtime UTF-8"),
        "--repository",
        repository.to_str().expect("repository UTF-8"),
    ];
    command.extend(args);
    let mut output = Vec::new();
    let mut error = Vec::new();
    let exit = run_cli(command, &mut output, &mut error);
    (
        exit,
        String::from_utf8(output).expect("stdout UTF-8"),
        String::from_utf8(error).expect("stderr UTF-8"),
    )
}
