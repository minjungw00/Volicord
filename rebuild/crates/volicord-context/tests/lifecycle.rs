use rusqlite::Connection;
use std::fs;
use std::path::Path;
use tempfile::tempdir;
use volicord_context::{
    AgentRecommendation, ApplicabilityScope, Availability, CanonicalRecordId,
    CanonicalRelationKind, ContextItemCorrectionDraft, ContextItemDraft, ContextItemRole,
    CorrectionKind, DecisionChoice, DecisionCorrectionDraft, DecisionSupersessionDraft,
    DeterministicIdGenerator, ErrorKind, ExplicitQuestionResponse, FixedClock, OperationId,
    Principal, PrincipalKind, Project, QuestionAlternative, QuestionDraft, QuestionResponseDraft,
    ReviewDueDraft, ReviewDueKind, Source, SourceDraft, SourcePayload, StatementProvenanceRole,
    Store, TimestampMicros, UserTurnSource,
};

fn operation(value: u8) -> OperationId {
    OperationId::from_bytes([value; 16])
}

fn test_store(path: &Path, ids: &[u8]) -> Result<Store, volicord_context::Error> {
    Store::open_with(
        path,
        DeterministicIdGenerator::new(ids.iter().map(|value| [*value; 16])),
        FixedClock::new(TimestampMicros::from_unix_micros(1_750_000_000_000_000)),
    )
}

fn principal(kind: PrincipalKind, identity: &str) -> Principal {
    Principal {
        kind,
        identity: identity.to_owned(),
    }
}

fn source_draft(payload: SourcePayload, actor: PrincipalKind) -> SourceDraft {
    SourceDraft {
        expected_project_revision: 1,
        payload,
        actor: principal(actor, "author"),
        observer: Some(principal(PrincipalKind::Agent, "codex")),
        availability: Availability::Available,
    }
}

fn user_turn(turn: &str) -> SourceDraft {
    source_draft(
        SourcePayload::CurrentHostUserTurn {
            host: "codex".to_owned(),
            session: "lifecycle".to_owned(),
            turn: turn.to_owned(),
        },
        PrincipalKind::User,
    )
}

fn setup(store: &mut Store) -> Result<(Project, Source, Source), volicord_context::Error> {
    let project = store.create_project(operation(1), "Lifecycle")?.value;
    let repository = store
        .record_source(
            operation(2),
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
    let user = store
        .record_source(operation(3), project.id, user_turn("authorization"))?
        .value;
    Ok((project, repository, user))
}

fn context_draft(repository: &Source, statement: &str) -> ContextItemDraft {
    ContextItemDraft {
        expected_project_revision: 1,
        role: ContextItemRole::Fact,
        statement: statement.to_owned(),
        provenance_role: StatementProvenanceRole::Observed,
        author: principal(PrincipalKind::Agent, "observer"),
        source_basis: vec![repository.id],
        applicability: ApplicabilityScope::default(),
    }
}

fn create_decision(
    store: &mut Store,
    project: &Project,
    repository: &Source,
) -> Result<volicord_context::Decision, Box<dyn std::error::Error>> {
    let question = store
        .create_question(
            operation(10),
            project.id,
            QuestionDraft {
                expected_project_revision: 1,
                prompt_basis: "Choose storage".to_owned(),
                source_basis: vec![repository.id],
                dependencies: vec![],
                alternatives: vec![
                    QuestionAlternative {
                        key: "local".to_owned(),
                        label: "Local".to_owned(),
                        consequence: "Keeps context local".to_owned(),
                    },
                    QuestionAlternative {
                        key: "remote".to_owned(),
                        label: "Remote".to_owned(),
                        consequence: "Transmits context".to_owned(),
                    },
                ],
                recommendation: AgentRecommendation {
                    alternative_key: Some("local".to_owned()),
                    rationale: "Local first".to_owned(),
                    source_basis: vec![repository.id],
                },
                trade_offs: vec!["availability".to_owned()],
                uncertainty: vec![],
                material_scope: vec!["storage".to_owned()],
                materiality: volicord_context::QuestionMateriality::Material,
                presentation_order: 0,
                why_it_matters_now: "storage authority is unresolved".to_owned(),
                established_facts: vec![],
                assumptions: vec![],
                known_limits: vec![],
                what_the_answer_unlocks: vec!["storage implementation".to_owned()],
                allowed_non_choice_dispositions: volicord_context::NonUserQuestionOutcome::ALL
                    .to_vec(),
                research_state: volicord_context::QuestionResearchState::ReadyToAsk,
            },
        )?
        .value;
    store
        .record_question_response(
            operation(11),
            project.id,
            QuestionResponseDraft {
                expected_project_revision: 1,
                question_id: question.id,
                question_revision: question.revision,
                user_turn_source: UserTurnSource::Create(user_turn("initial-decision")),
                displayed_alternative_keys: vec!["local".to_owned(), "remote".to_owned()],
                displayed_recommendation_key: Some("local".to_owned()),
                response: ExplicitQuestionResponse::Choice {
                    alternative_key: "local".to_owned(),
                    user_rationale: Some("Keep data local".to_owned()),
                },
                applicability: ApplicabilityScope::default(),
                assumptions: vec!["local disk available".to_owned()],
                revisit_triggers: vec!["disk unavailable".to_owned()],
            },
        )?
        .value
        .decision
        .ok_or_else(|| "missing Decision".into())
}

#[test]
fn correction_preserves_identity_history_and_rejects_semantic_change(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let path = root.path().join("context.sqlite3");
    let mut store = test_store(&path, &[1, 2, 3, 4])?;
    let (project, repository, user) = setup(&mut store)?;
    let item = store
        .record_context_item(
            operation(4),
            project.id,
            context_draft(&repository, "The policy is local."),
        )?
        .value;
    let corrected = store
        .correct_context_item(
            operation(5),
            project.id,
            item.id,
            ContextItemCorrectionDraft {
                expected_revision: 1,
                corrected_statement: "The  policy is local.".to_owned(),
                kind: CorrectionKind::Formatting,
                user_authorization_source_id: user.id,
            },
        )?
        .value;
    assert_eq!(corrected.id, item.id);
    assert_eq!(corrected.revision, 2);
    let history = store.get_context_item_history(project.id, item.id)?;
    assert_eq!(
        history
            .iter()
            .map(|value| value.revision)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(history[0].statement, "The policy is local.");
    let error = store
        .correct_context_item(
            operation(6),
            project.id,
            item.id,
            ContextItemCorrectionDraft {
                expected_revision: 2,
                corrected_statement: "The policy is remote.".to_owned(),
                kind: CorrectionKind::Expression,
                user_authorization_source_id: user.id,
            },
        )
        .err()
        .ok_or("expected semantic correction rejection")?;
    assert_eq!(error.kind(), ErrorKind::DomainConflict);
    drop(store);
    let reopened = test_store(&path, &[])?;
    assert_eq!(
        reopened
            .get_context_item_history(project.id, item.id)?
            .len(),
        2
    );
    Ok(())
}

#[test]
fn decision_supersession_is_directed_and_current_selection_is_deterministic(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut store = test_store(
        &root.path().join("context.sqlite3"),
        &[1, 2, 3, 4, 5, 6, 7, 8],
    )?;
    let (project, repository, user) = setup(&mut store)?;
    let first = create_decision(&mut store, &project, &repository)?;
    let semantic_overwrite = store
        .correct_decision(
            operation(12),
            project.id,
            first.id,
            DecisionCorrectionDraft {
                expected_revision: 1,
                corrected_user_rationale: Some("Send data remote".to_owned()),
                kind: CorrectionKind::Expression,
                user_authorization_source_id: user.id,
            },
        )
        .err()
        .ok_or("expected Decision semantic correction rejection")?;
    assert_eq!(semantic_overwrite.kind(), ErrorKind::DomainConflict);
    let second = store
        .supersede_decision(
            operation(13),
            project.id,
            DecisionSupersessionDraft {
                expected_project_revision: 1,
                previous_decision_id: first.id,
                user_turn_source: UserTurnSource::Create(user_turn("changed-decision")),
                choice: DecisionChoice::Alternative {
                    alternative_key: "remote".to_owned(),
                },
                user_rationale: Some("Capability now required".to_owned()),
                applicability: ApplicabilityScope::default(),
                assumptions: vec!["explicit opt-in".to_owned()],
                revisit_triggers: vec!["provider changes".to_owned()],
            },
        )?
        .value;
    assert_ne!(first.id, second.id);
    assert_eq!(
        store
            .get_canonical_relation(
                project.id,
                CanonicalRecordId::Decision(second.id),
                CanonicalRelationKind::Supersedes,
                CanonicalRecordId::Decision(first.id)
            )?
            .to,
        CanonicalRecordId::Decision(first.id)
    );
    let current = store.get_current_decision(project.id, first.question_id)?;
    assert_eq!(current.decision.id, second.id);
    assert_eq!(
        store
            .get_decision_history(project.id, first.question_id)?
            .iter()
            .map(|value| value.id)
            .collect::<Vec<_>>(),
        vec![first.id, second.id]
    );
    assert!(
        store
            .supersede_decision(
                operation(13),
                project.id,
                DecisionSupersessionDraft {
                    expected_project_revision: 1,
                    previous_decision_id: first.id,
                    user_turn_source: UserTurnSource::Create(user_turn("changed-decision")),
                    choice: DecisionChoice::Alternative {
                        alternative_key: "remote".to_owned()
                    },
                    user_rationale: Some("Capability now required".to_owned()),
                    applicability: ApplicabilityScope::default(),
                    assumptions: vec!["explicit opt-in".to_owned()],
                    revisit_triggers: vec!["provider changes".to_owned()]
                },
            )?
            .replayed
    );
    Ok(())
}

#[test]
fn contradiction_keeps_both_sources_and_review_due_does_not_invalidate(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut store = test_store(&root.path().join("context.sqlite3"), &[1, 2, 3, 4, 5, 6, 7])?;
    let (project, repository, _) = setup(&mut store)?;
    let decision = create_decision(&mut store, &project, &repository)?;
    let conflicting = store
        .record_context_item(
            operation(20),
            project.id,
            context_draft(&repository, "The provider is required"),
        )?
        .value;
    store.record_contradiction(
        operation(21),
        project.id,
        CanonicalRecordId::Decision(decision.id),
        CanonicalRecordId::ContextItem(conflicting.id),
    )?;
    store.mark_decision_review_due(
        operation(22),
        project.id,
        decision.id,
        ReviewDueDraft {
            kind: ReviewDueKind::SourceFreshnessChanged,
            explanation: "Repository evidence changed".to_owned(),
            source_basis: vec![repository.id],
        },
    )?;
    let lifecycle = store.get_decision_lifecycle(project.id, decision.id)?;
    assert_eq!(lifecycle.decision.id, decision.id);
    assert_eq!(
        lifecycle.contradictions,
        vec![CanonicalRecordId::ContextItem(conflicting.id)]
    );
    assert_eq!(
        lifecycle.review_due.ok_or("missing review due")?.kind,
        ReviewDueKind::SourceFreshnessChanged
    );
    assert_eq!(
        store
            .get_context_item(project.id, conflicting.id)?
            .source_basis,
        vec![repository.id]
    );
    assert_eq!(
        store
            .get_decision(project.id, decision.id)?
            .displayed_recommendation
            .source_basis,
        vec![repository.id]
    );
    Ok(())
}

#[test]
fn forgetting_requires_user_authority_removes_content_and_leaves_minimal_residue(
) -> Result<(), Box<dyn std::error::Error>> {
    const SECRET: &str = "FORGET-ME-raw-private-value-9973";
    let root = tempdir()?;
    let path = root.path().join("context.sqlite3");
    let mut store = test_store(&path, &[1, 2, 3, 4])?;
    let (project, repository, user) = setup(&mut store)?;
    let item = store
        .record_context_item(
            operation(30),
            project.id,
            context_draft(&repository, SECRET),
        )?
        .value;
    let unauthorized = store
        .forget_context_item(operation(31), project.id, item.id, repository.id)
        .err()
        .ok_or("expected authority rejection")?;
    assert_eq!(unauthorized.kind(), ErrorKind::InvalidInput);
    let forgotten = store.forget_context_item(operation(32), project.id, item.id, user.id)?;
    assert_eq!(
        forgotten.value.tombstone.record,
        CanonicalRecordId::ContextItem(item.id)
    );
    assert_eq!(
        forgotten.value.invalidation.record,
        CanonicalRecordId::ContextItem(item.id)
    );
    assert_eq!(
        store
            .get_context_item(project.id, item.id)
            .err()
            .ok_or("forgotten item should be unreadable")?
            .kind(),
        ErrorKind::NotFound
    );
    assert!(
        store
            .forget_context_item(operation(32), project.id, item.id, user.id)?
            .replayed
    );
    drop(store);
    let reopened = test_store(&path, &[])?;
    assert_eq!(
        reopened
            .get_tombstone(project.id, CanonicalRecordId::ContextItem(item.id))?
            .record,
        CanonicalRecordId::ContextItem(item.id)
    );
    assert_eq!(
        reopened
            .get_context_item(project.id, item.id)
            .err()
            .ok_or("forgotten item should stay unreadable")?
            .kind(),
        ErrorKind::NotFound
    );
    drop(reopened);
    for entry in fs::read_dir(root.path())? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let bytes = fs::read(entry.path())?;
            assert!(
                !bytes
                    .windows(SECRET.len())
                    .any(|window| window == SECRET.as_bytes()),
                "forgotten bytes remain in {}",
                entry.path().display()
            );
        }
    }
    let connection = Connection::open(&path)?;
    let tombstone_columns: Vec<String> = connection
        .prepare("PRAGMA table_info(tombstones)")?
        .query_map([], |row| row.get(1))?
        .collect::<Result<_, _>>()?;
    assert_eq!(
        tombstone_columns,
        vec!["project_id", "record_kind", "record_id", "forgotten_at"]
    );
    assert!(!tombstone_columns
        .iter()
        .any(|column| column.contains("content") || column.contains("hash")));
    Ok(())
}

#[test]
fn forgetting_failure_rolls_back_and_operation_can_be_retried(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let path = root.path().join("context.sqlite3");
    let mut store = test_store(&path, &[1, 2, 3, 4])?;
    let (project, repository, user) = setup(&mut store)?;
    let item = store
        .record_context_item(
            operation(40),
            project.id,
            context_draft(&repository, "retain after rollback"),
        )?
        .value;
    drop(store);
    let connection = Connection::open(&path)?;
    connection.execute_batch("CREATE TRIGGER reject_forget BEFORE DELETE ON context_items BEGIN SELECT RAISE(ABORT, 'fault'); END;")?;
    drop(connection);
    let mut store = test_store(&path, &[])?;
    assert_eq!(
        store
            .forget_context_item(operation(41), project.id, item.id, user.id)
            .err()
            .ok_or("expected injected failure")?
            .kind(),
        ErrorKind::TransactionFailure
    );
    assert_eq!(
        store.get_context_item(project.id, item.id)?.statement,
        "retain after rollback"
    );
    assert_eq!(
        store
            .get_tombstone(project.id, CanonicalRecordId::ContextItem(item.id))
            .err()
            .ok_or("tombstone must roll back")?
            .kind(),
        ErrorKind::NotFound
    );
    drop(store);
    let connection = Connection::open(&path)?;
    connection.execute_batch("DROP TRIGGER reject_forget;")?;
    drop(connection);
    let mut store = test_store(&path, &[])?;
    assert!(
        !store
            .forget_context_item(operation(41), project.id, item.id, user.id)?
            .replayed
    );
    Ok(())
}
