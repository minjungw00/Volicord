use rusqlite::Connection;
use std::path::Path;
use tempfile::tempdir;
use volicord_context::{
    AgentRecommendation, ApplicabilityScope, Availability, DecisionChoice,
    DeterministicIdGenerator, ErrorKind, ExplicitQuestionResponse, FixedClock, OperationId,
    Principal, PrincipalKind, Project, Question, QuestionAlternative, QuestionDraft,
    QuestionResponseDraft, QuestionState, QuestionTerminalOutcome, Source, SourceDraft,
    SourcePayload, Store, TimestampMicros, UserTurnSource,
};

fn operation(value: u8) -> OperationId {
    OperationId::from_bytes([value; 16])
}

fn store_with_ids(path: &Path, values: &[u8]) -> Result<Store, volicord_context::Error> {
    Store::open_with(
        path,
        DeterministicIdGenerator::new(values.iter().map(|value| [*value; 16])),
        FixedClock::new(TimestampMicros::from_unix_micros(1_735_689_600_000_000)),
    )
}

fn principal(kind: PrincipalKind, identity: &str) -> Principal {
    Principal {
        kind,
        identity: identity.to_owned(),
    }
}

fn source_draft(payload: SourcePayload, actor_kind: PrincipalKind) -> SourceDraft {
    SourceDraft {
        expected_project_revision: 1,
        payload,
        actor: principal(actor_kind, "actor"),
        observer: Some(principal(PrincipalKind::Agent, "codex-session")),
        availability: Availability::Available,
    }
}

fn user_turn(turn: &str) -> SourceDraft {
    source_draft(
        SourcePayload::CurrentHostUserTurn {
            host: "codex".to_owned(),
            session: "session-1".to_owned(),
            turn: turn.to_owned(),
        },
        PrincipalKind::User,
    )
}

fn setup_question(
    store: &mut Store,
    project: &Project,
    operation_base: u8,
) -> Result<(Source, Question), volicord_context::Error> {
    let basis = store
        .record_source(
            operation(operation_base),
            project.id,
            source_draft(
                SourcePayload::File {
                    locator: "src/policy.rs".to_owned(),
                    snapshot: "commit-a".to_owned(),
                },
                PrincipalKind::Repository,
            ),
        )?
        .value;
    let question = store
        .create_question(
            operation(operation_base + 1),
            project.id,
            QuestionDraft {
                expected_project_revision: 1,
                prompt_basis: "Choose the persistence policy".to_owned(),
                source_basis: vec![basis.id],
                dependencies: vec![],
                alternatives: vec![
                    QuestionAlternative {
                        key: "local".to_owned(),
                        label: "Local only".to_owned(),
                        consequence: "No background transmission".to_owned(),
                    },
                    QuestionAlternative {
                        key: "remote".to_owned(),
                        label: "Remote provider".to_owned(),
                        consequence: "Requires explicit opt-in".to_owned(),
                    },
                ],
                recommendation: AgentRecommendation {
                    alternative_key: Some("local".to_owned()),
                    rationale: "Preserves the local-first boundary".to_owned(),
                    source_basis: vec![basis.id],
                },
                trade_offs: vec!["Capability versus disclosure".to_owned()],
                uncertainty: vec!["Future provider availability".to_owned()],
                material_scope: vec!["canonical storage".to_owned()],
            },
        )?
        .value;
    Ok((basis, question))
}

fn response(
    question: &Question,
    source: UserTurnSource,
    explicit: ExplicitQuestionResponse,
) -> QuestionResponseDraft {
    QuestionResponseDraft {
        expected_project_revision: 1,
        question_id: question.id,
        question_revision: question.revision,
        user_turn_source: source,
        displayed_alternative_keys: question
            .alternatives
            .iter()
            .map(|alternative| alternative.key.clone())
            .collect(),
        displayed_recommendation_key: question.recommendation.alternative_key.clone(),
        response: explicit,
        applicability: ApplicabilityScope {
            paths: vec!["rebuild/".to_owned()],
            components: vec!["Canonical Context Kernel".to_owned()],
            work_contexts: vec!["initial implementation".to_owned()],
        },
        assumptions: vec!["local storage remains available".to_owned()],
        revisit_triggers: vec!["provider policy changes".to_owned()],
    }
}

#[test]
fn records_choice_with_exact_revision_and_separate_recommendation(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut store = store_with_ids(&root.path().join("context.sqlite3"), &[1, 2, 3, 4, 5])?;
    let project = store.create_project(operation(1), "Inquiry")?.value;
    let (_, question) = setup_question(&mut store, &project, 2)?;
    let result = store
        .record_question_response(
            operation(4),
            project.id,
            response(
                &question,
                UserTurnSource::Create(user_turn("turn-choice")),
                ExplicitQuestionResponse::Choice {
                    alternative_key: "remote".to_owned(),
                    user_rationale: Some("The capability is needed".to_owned()),
                },
            ),
        )?
        .value;
    assert_eq!(
        result.question.state,
        QuestionState::Terminal(QuestionTerminalOutcome::Answered)
    );
    let decision = result.decision.ok_or("expected Decision")?;
    assert_eq!(
        decision.choice,
        DecisionChoice::Alternative {
            alternative_key: "remote".to_owned()
        }
    );
    assert_eq!(
        decision.displayed_recommendation.alternative_key.as_deref(),
        Some("local")
    );
    assert_eq!(decision.question_revision, 1);
    assert_eq!(store.get_decision(project.id, decision.id)?, decision);
    Ok(())
}

#[test]
fn records_explicit_delegation_as_a_decision() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut store = store_with_ids(&root.path().join("context.sqlite3"), &[1, 2, 3, 4, 5])?;
    let project = store.create_project(operation(10), "Delegation")?.value;
    let (_, question) = setup_question(&mut store, &project, 11)?;
    let result = store.record_question_response(
        operation(13),
        project.id,
        response(
            &question,
            UserTurnSource::Create(user_turn("turn-delegate")),
            ExplicitQuestionResponse::Delegation {
                delegate_to: "implementation agent".to_owned(),
                user_rationale: None,
            },
        ),
    )?;
    assert_eq!(
        result.value.question.state,
        QuestionState::Terminal(QuestionTerminalOutcome::Delegated)
    );
    assert_eq!(
        result.value.decision.ok_or("expected Decision")?.choice,
        DecisionChoice::Delegation {
            delegate_to: "implementation agent".to_owned()
        }
    );
    Ok(())
}

#[test]
fn records_every_non_decision_terminal_outcome_without_a_decision(
) -> Result<(), Box<dyn std::error::Error>> {
    let outcomes = [
        QuestionTerminalOutcome::ResolvedByResearch,
        QuestionTerminalOutcome::RequiresPrototype,
        QuestionTerminalOutcome::Deferred,
        QuestionTerminalOutcome::OutOfScope,
        QuestionTerminalOutcome::Superseded,
    ];
    for (index, outcome) in outcomes.into_iter().enumerate() {
        let root = tempdir()?;
        let mut store = store_with_ids(&root.path().join("context.sqlite3"), &[1, 2, 3, 4])?;
        let project = store.create_project(operation(20), "Terminal")?.value;
        let (_, question) = setup_question(&mut store, &project, 21)?;
        let result = store.record_question_response(
            operation(23),
            project.id,
            response(
                &question,
                UserTurnSource::Create(user_turn(&format!("turn-{index}"))),
                ExplicitQuestionResponse::Terminal { outcome },
            ),
        )?;
        assert_eq!(
            result.value.question.state,
            QuestionState::Terminal(outcome)
        );
        assert!(result.value.decision.is_none());
    }
    Ok(())
}

#[test]
fn rejects_stale_wrong_project_missing_provenance_terminal_and_mismatched_display(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut store = store_with_ids(&root.path().join("context.sqlite3"), &[1, 2, 3, 4, 5, 6, 7])?;
    let project = store.create_project(operation(30), "First")?.value;
    let other = store.create_project(operation(31), "Other")?.value;
    let (_, question) = setup_question(&mut store, &project, 32)?;

    let mut stale = response(
        &question,
        UserTurnSource::Create(user_turn("stale")),
        ExplicitQuestionResponse::Choice {
            alternative_key: "local".to_owned(),
            user_rationale: None,
        },
    );
    stale.question_revision = 2;
    assert_eq!(
        store
            .record_question_response(operation(34), project.id, stale)
            .err()
            .ok_or("expected stale revision")?
            .kind(),
        ErrorKind::StaleBasis
    );

    let wrong = response(
        &question,
        UserTurnSource::Create(user_turn("wrong-project")),
        ExplicitQuestionResponse::Choice {
            alternative_key: "local".to_owned(),
            user_rationale: None,
        },
    );
    assert_eq!(
        store
            .record_question_response(operation(35), other.id, wrong)
            .err()
            .ok_or("expected wrong Project")?
            .kind(),
        ErrorKind::WrongProject
    );

    let agent_turn = source_draft(
        SourcePayload::CurrentHostUserTurn {
            host: "codex".to_owned(),
            session: "session".to_owned(),
            turn: "agent-turn".to_owned(),
        },
        PrincipalKind::Agent,
    );
    let missing = response(
        &question,
        UserTurnSource::Create(agent_turn),
        ExplicitQuestionResponse::Choice {
            alternative_key: "local".to_owned(),
            user_rationale: None,
        },
    );
    assert_eq!(
        store
            .record_question_response(operation(36), project.id, missing)
            .err()
            .ok_or("expected provenance rejection")?
            .kind(),
        ErrorKind::InvalidInput
    );

    let mut mismatch = response(
        &question,
        UserTurnSource::Create(user_turn("mismatch")),
        ExplicitQuestionResponse::Choice {
            alternative_key: "local".to_owned(),
            user_rationale: None,
        },
    );
    mismatch.displayed_alternative_keys.swap(0, 1);
    assert_eq!(
        store
            .record_question_response(operation(37), project.id, mismatch)
            .err()
            .ok_or("expected displayed basis mismatch")?
            .kind(),
        ErrorKind::StaleBasis
    );

    store.record_question_response(
        operation(38),
        project.id,
        response(
            &question,
            UserTurnSource::Create(user_turn("valid")),
            ExplicitQuestionResponse::Choice {
                alternative_key: "local".to_owned(),
                user_rationale: None,
            },
        ),
    )?;
    assert_eq!(
        store
            .record_question_response(
                operation(39),
                project.id,
                response(
                    &question,
                    UserTurnSource::Create(user_turn("too-late")),
                    ExplicitQuestionResponse::Choice {
                        alternative_key: "local".to_owned(),
                        user_rationale: None,
                    },
                ),
            )
            .err()
            .ok_or("expected terminal rejection")?
            .kind(),
        ErrorKind::DomainConflict
    );
    Ok(())
}

#[test]
fn response_transaction_rolls_back_at_every_intermediate_write_boundary(
) -> Result<(), Box<dyn std::error::Error>> {
    let faults = [
        ("sources", "BEFORE INSERT ON sources"),
        ("response", "BEFORE INSERT ON question_response_sources"),
        ("decision", "BEFORE INSERT ON decisions"),
        (
            "question",
            "BEFORE UPDATE ON questions WHEN NEW.terminal_outcome IS NOT NULL",
        ),
        (
            "operation",
            "BEFORE INSERT ON operations WHEN NEW.operation_kind = 'record_question_response'",
        ),
    ];
    for (index, (name, clause)) in faults.into_iter().enumerate() {
        let root = tempdir()?;
        let path = root.path().join("context.sqlite3");
        let mut store = store_with_ids(&path, &[1, 2, 3])?;
        let project = store.create_project(operation(40), "Rollback")?.value;
        let (_, question) = setup_question(&mut store, &project, 41)?;
        drop(store);
        let connection = Connection::open(&path)?;
        connection.execute_batch(&format!(
            "CREATE TRIGGER fail_{name} {clause} BEGIN SELECT RAISE(ABORT, 'fault'); END;"
        ))?;
        drop(connection);

        let mut store = store_with_ids(&path, &[4, 5])?;
        let error = store
            .record_question_response(
                operation(43),
                project.id,
                response(
                    &question,
                    UserTurnSource::Create(user_turn(&format!("rollback-{index}"))),
                    ExplicitQuestionResponse::Choice {
                        alternative_key: "local".to_owned(),
                        user_rationale: None,
                    },
                ),
            )
            .err()
            .ok_or("expected injected failure")?;
        assert_eq!(error.kind(), ErrorKind::TransactionFailure, "fault {name}");
        assert_eq!(
            store.get_question(project.id, question.id)?.state,
            QuestionState::Open,
            "fault {name}"
        );
        drop(store);
        let connection = Connection::open(&path)?;
        let decision_count: i64 =
            connection.query_row("SELECT count(*) FROM decisions", [], |row| row.get(0))?;
        let response_count: i64 = connection.query_row(
            "SELECT count(*) FROM question_response_sources",
            [],
            |row| row.get(0),
        )?;
        let response_operation_count: i64 = connection.query_row(
            "SELECT count(*) FROM operations WHERE operation_kind = 'record_question_response'",
            [],
            |row| row.get(0),
        )?;
        let user_source_count: i64 = connection.query_row(
            "SELECT count(*) FROM sources WHERE source_kind = 'current_host_user_turn'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(decision_count, 0, "fault {name}");
        assert_eq!(response_count, 0, "fault {name}");
        assert_eq!(response_operation_count, 0, "fault {name}");
        assert_eq!(user_source_count, 0, "fault {name}");
    }
    Ok(())
}

#[test]
fn committed_response_replays_after_restart_and_rejects_changed_input(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let path = root.path().join("context.sqlite3");
    let mut store = store_with_ids(&path, &[1, 2, 3, 4, 5])?;
    let project = store.create_project(operation(50), "Replay")?.value;
    let (_, question) = setup_question(&mut store, &project, 51)?;
    let draft = response(
        &question,
        UserTurnSource::Create(user_turn("replayed-turn")),
        ExplicitQuestionResponse::Choice {
            alternative_key: "local".to_owned(),
            user_rationale: Some("Keep it local".to_owned()),
        },
    );
    let committed = store.record_question_response(operation(53), project.id, draft.clone())?;
    drop(store);

    let mut reopened = store_with_ids(&path, &[])?;
    let replay = reopened.record_question_response(operation(53), project.id, draft.clone())?;
    assert!(replay.replayed);
    assert_eq!(replay.value, committed.value);
    let mut changed = draft;
    changed.assumptions.push("changed input".to_owned());
    assert_eq!(
        reopened
            .record_question_response(operation(53), project.id, changed)
            .err()
            .ok_or("expected operation mismatch")?
            .kind(),
        ErrorKind::DomainConflict
    );
    Ok(())
}

#[test]
fn one_user_turn_links_to_multiple_questions_only_through_explicit_responses(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut store = store_with_ids(
        &root.path().join("context.sqlite3"),
        &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    )?;
    let project = store.create_project(operation(60), "Multiple")?.value;
    let (_, first) = setup_question(&mut store, &project, 61)?;
    let (_, second) = setup_question(&mut store, &project, 63)?;
    let (_, untouched) = setup_question(&mut store, &project, 65)?;
    let turn = store
        .record_source(operation(67), project.id, user_turn("multi-turn"))?
        .value;
    let first_result = store.record_question_response(
        operation(68),
        project.id,
        response(
            &first,
            UserTurnSource::Existing(turn.id),
            ExplicitQuestionResponse::Choice {
                alternative_key: "local".to_owned(),
                user_rationale: None,
            },
        ),
    )?;
    let second_result = store.record_question_response(
        operation(69),
        project.id,
        response(
            &second,
            UserTurnSource::Existing(turn.id),
            ExplicitQuestionResponse::Choice {
                alternative_key: "remote".to_owned(),
                user_rationale: None,
            },
        ),
    )?;
    assert_eq!(first_result.value.user_turn_source.id, turn.id);
    assert_eq!(second_result.value.user_turn_source.id, turn.id);
    assert_eq!(
        store.get_question(project.id, untouched.id)?.state,
        QuestionState::Open
    );
    Ok(())
}
