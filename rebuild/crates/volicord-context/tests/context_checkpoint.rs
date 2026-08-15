use rusqlite::Connection;
use std::path::Path;
use tempfile::tempdir;
use volicord_context::{
    AgentRecommendation, ApplicabilityScope, Availability, CheckpointDraft, CheckpointKind,
    ContextItemDraft, ContextItemRole, DeterministicIdGenerator, ErrorKind,
    ExplicitQuestionResponse, FixedClock, OperationId, Principal, PrincipalKind, Project,
    QuestionAlternative, QuestionDraft, QuestionReference, QuestionResponseDraft, Source,
    SourceDraft, SourcePayload, StatementProvenanceRole, Store, TimestampMicros,
    UserAcceptanceFact, UserAcceptanceState, UserReviewFact, UserReviewState, UserTurnSource,
    VerificationFact, VerificationState, WorkState,
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

fn draft(payload: SourcePayload, actor: PrincipalKind) -> SourceDraft {
    SourceDraft {
        expected_project_revision: 1,
        payload,
        actor: principal(actor, "source-actor"),
        observer: Some(principal(PrincipalKind::Agent, "codex")),
        availability: Availability::Available,
    }
}

fn record_sources(
    store: &mut Store,
    project: &Project,
) -> Result<(Source, Source, Source, Source), volicord_context::Error> {
    let repository = store
        .record_source(
            operation(2),
            project.id,
            draft(
                SourcePayload::File {
                    locator: "src/lib.rs".to_owned(),
                    snapshot: "commit-a".to_owned(),
                },
                PrincipalKind::Repository,
            ),
        )?
        .value;
    let user = store
        .record_source(
            operation(3),
            project.id,
            draft(
                SourcePayload::CurrentHostUserTurn {
                    host: "codex".to_owned(),
                    session: "session".to_owned(),
                    turn: "turn-1".to_owned(),
                },
                PrincipalKind::User,
            ),
        )?
        .value;
    let generated = store
        .record_source(
            operation(4),
            project.id,
            draft(
                SourcePayload::AdoptedArtifact {
                    locator: "generated/analysis.md".to_owned(),
                    revision: "generation-1".to_owned(),
                },
                PrincipalKind::Generator,
            ),
        )?
        .value;
    let command = store
        .record_source(
            operation(5),
            project.id,
            draft(
                SourcePayload::CommandExecution {
                    command_label: "cargo test".to_owned(),
                    outcome: volicord_context::CommandOutcome {
                        exit_code: Some(0),
                        termination: volicord_context::CommandTermination::Exited,
                    },
                },
                PrincipalKind::Command,
            ),
        )?
        .value;
    Ok((repository, user, generated, command))
}

fn context_draft(
    role: ContextItemRole,
    provenance_role: StatementProvenanceRole,
    author: PrincipalKind,
    source_id: volicord_context::SourceId,
) -> ContextItemDraft {
    ContextItemDraft {
        expected_project_revision: 1,
        role,
        statement: format!("statement for {role:?}"),
        provenance_role,
        author: principal(author, "item-author"),
        source_basis: vec![source_id],
        applicability: ApplicabilityScope {
            paths: vec!["rebuild/".to_owned()],
            components: vec!["context".to_owned()],
            work_contexts: vec!["implementation".to_owned()],
        },
    }
}

#[test]
fn records_every_context_role_without_reclassifying_provenance(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut store = store_with_ids(
        &root.path().join("context.sqlite3"),
        &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
    )?;
    let project = store.create_project(operation(1), "Context")?.value;
    let (repository, user, generated, _) = record_sources(&mut store, &project)?;
    let cases = [
        (
            ContextItemRole::Goal,
            StatementProvenanceRole::UserStatement,
            PrincipalKind::User,
            user.id,
        ),
        (
            ContextItemRole::Fact,
            StatementProvenanceRole::Observed,
            PrincipalKind::Agent,
            repository.id,
        ),
        (
            ContextItemRole::Assumption,
            StatementProvenanceRole::AgentStatement,
            PrincipalKind::Agent,
            repository.id,
        ),
        (
            ContextItemRole::Constraint,
            StatementProvenanceRole::UserStatement,
            PrincipalKind::User,
            user.id,
        ),
        (
            ContextItemRole::Preference,
            StatementProvenanceRole::UserStatement,
            PrincipalKind::User,
            user.id,
        ),
        (
            ContextItemRole::Risk,
            StatementProvenanceRole::GeneratedInterpretation,
            PrincipalKind::Generator,
            generated.id,
        ),
        (
            ContextItemRole::Learning,
            StatementProvenanceRole::Observed,
            PrincipalKind::Agent,
            repository.id,
        ),
        (
            ContextItemRole::KnownLimit,
            StatementProvenanceRole::AgentStatement,
            PrincipalKind::Agent,
            repository.id,
        ),
    ];
    for (index, (role, provenance, author, source)) in cases.into_iter().enumerate() {
        let item = store
            .record_context_item(
                operation(10 + index as u8),
                project.id,
                context_draft(role, provenance, author, source),
            )?
            .value;
        assert_eq!(item.role, role);
        assert_eq!(item.provenance_role, provenance);
        assert_eq!(item.source_basis, vec![source]);
        assert_eq!(store.get_context_item(project.id, item.id)?, item);
    }
    Ok(())
}

#[test]
fn rejects_generated_fact_and_non_user_preference_and_keeps_support_direction(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let path = root.path().join("context.sqlite3");
    let mut store = store_with_ids(&path, &[1, 2, 3, 4, 5, 6])?;
    let project = store.create_project(operation(20), "Provenance")?.value;
    let (repository, _, generated, _) = record_sources(&mut store, &project)?;
    let generated_fact = context_draft(
        ContextItemRole::Fact,
        StatementProvenanceRole::GeneratedInterpretation,
        PrincipalKind::Generator,
        generated.id,
    );
    assert_eq!(
        store
            .record_context_item(operation(21), project.id, generated_fact)
            .err()
            .ok_or("expected generated fact rejection")?
            .kind(),
        ErrorKind::InvalidInput
    );
    let preference = context_draft(
        ContextItemRole::Preference,
        StatementProvenanceRole::AgentStatement,
        PrincipalKind::Agent,
        repository.id,
    );
    assert_eq!(
        store
            .record_context_item(operation(22), project.id, preference)
            .err()
            .ok_or("expected preference provenance rejection")?
            .kind(),
        ErrorKind::InvalidInput
    );
    let fact = store
        .record_context_item(
            operation(23),
            project.id,
            context_draft(
                ContextItemRole::Fact,
                StatementProvenanceRole::Observed,
                PrincipalKind::Agent,
                repository.id,
            ),
        )?
        .value;
    drop(store);
    let connection = Connection::open(&path)?;
    let (from_item, to_source): (Vec<u8>, Vec<u8>) = connection.query_row(
        "SELECT context_item_id, source_id FROM context_item_sources",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    assert_eq!(from_item, fact.id.as_bytes());
    assert_eq!(to_source, repository.id.as_bytes());
    Ok(())
}

#[test]
fn rejects_agent_provenance_presented_as_a_user_goal() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut store = store_with_ids(&root.path().join("context.sqlite3"), &[1, 2, 3, 4, 5, 6])?;
    let project = store
        .create_project(operation(24), "User Goal Provenance")?
        .value;
    let (repository, _, _, _) = record_sources(&mut store, &project)?;
    let forged_goal = context_draft(
        ContextItemRole::Goal,
        StatementProvenanceRole::UserStatement,
        PrincipalKind::Agent,
        repository.id,
    );
    let error = store
        .record_context_item(operation(25), project.id, forged_goal)
        .expect_err("agent provenance must not become a user-stated Goal");
    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert!(error.to_string().contains("user provenance"));
    Ok(())
}

fn question_draft(source_id: volicord_context::SourceId, prompt: &str) -> QuestionDraft {
    QuestionDraft {
        expected_project_revision: 1,
        prompt_basis: prompt.to_owned(),
        source_basis: vec![source_id],
        dependencies: vec![],
        alternatives: vec![QuestionAlternative {
            key: "yes".to_owned(),
            label: "Yes".to_owned(),
            consequence: "Proceed".to_owned(),
        }],
        recommendation: AgentRecommendation {
            alternative_key: Some("yes".to_owned()),
            rationale: "Evidence supports proceeding".to_owned(),
            source_basis: vec![source_id],
        },
        trade_offs: vec!["time".to_owned()],
        uncertainty: vec!["future changes".to_owned()],
        material_scope: vec!["checkpoint tests".to_owned()],
        materiality: volicord_context::QuestionMateriality::Material,
        presentation_order: 0,
        why_it_matters_now: "checkpoint work depends on this choice".to_owned(),
        established_facts: vec![],
        assumptions: vec![],
        known_limits: vec![],
        what_the_answer_unlocks: vec!["checkpoint progress".to_owned()],
        allowed_non_choice_dispositions: volicord_context::NonUserQuestionOutcome::ALL.to_vec(),
        research_state: volicord_context::QuestionResearchState::ReadyToAsk,
    }
}

fn checkpoint_draft(
    kind: CheckpointKind,
    work_state: WorkState,
    source_id: volicord_context::SourceId,
) -> CheckpointDraft {
    CheckpointDraft {
        expected_project_revision: 1,
        kind,
        goal: "Implement durable context".to_owned(),
        work_state,
        state_change: None,
        source_basis: vec![source_id],
        changed_source_basis: vec![],
        changed_paths: vec![],
        applied_decisions: vec![],
        verification: vec![VerificationFact {
            state: VerificationState::NotRun,
            source_id: None,
            outcome: None,
        }],
        user_review: UserReviewFact {
            state: UserReviewState::NotRequested,
            source_id: None,
        },
        user_acceptance: UserAcceptanceFact {
            state: UserAcceptanceState::NotRequested,
            source_id: None,
        },
        known_limits: vec![],
        non_goals: vec!["No repository inference".to_owned()],
        open_questions: vec![],
        next_step: "Continue with the recorded basis".to_owned(),
        handoff_to: None,
    }
}

#[test]
fn records_completion_with_independent_states_and_explicit_relations(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut store = store_with_ids(
        &root.path().join("context.sqlite3"),
        &[1, 2, 3, 4, 5, 6, 7, 8, 9],
    )?;
    let project = store.create_project(operation(30), "Checkpoint")?.value;
    let (repository, user, _, command) = record_sources(&mut store, &project)?;
    let answered = store
        .create_question(
            operation(31),
            project.id,
            question_draft(repository.id, "Apply the change?"),
        )?
        .value;
    let decision = store
        .record_question_response(
            operation(32),
            project.id,
            QuestionResponseDraft {
                expected_project_revision: 1,
                question_id: answered.id,
                question_revision: 1,
                user_turn_source: UserTurnSource::Existing(user.id),
                displayed_alternative_keys: vec!["yes".to_owned()],
                displayed_recommendation_key: Some("yes".to_owned()),
                response: ExplicitQuestionResponse::Choice {
                    alternative_key: "yes".to_owned(),
                    user_rationale: Some("Apply now".to_owned()),
                },
                applicability: ApplicabilityScope::default(),
                assumptions: vec![],
                revisit_triggers: vec![],
            },
        )?
        .value
        .decision
        .ok_or("expected Decision")?;
    let open = store
        .create_question(
            operation(33),
            project.id,
            question_draft(repository.id, "What comes next?"),
        )?
        .value;
    let mut draft = checkpoint_draft(
        CheckpointKind::Completion,
        WorkState::Completed,
        repository.id,
    );
    draft.state_change = Some("Context persistence implemented".to_owned());
    draft.changed_source_basis = vec![repository.id];
    draft.changed_paths = vec!["rebuild/crates/volicord-context/src/store.rs".to_owned()];
    draft.applied_decisions = vec![decision.id];
    draft.verification = vec![VerificationFact {
        state: VerificationState::Failed,
        source_id: Some(command.id),
        outcome: Some("one focused assertion failed".to_owned()),
    }];
    draft.user_review = UserReviewFact {
        state: UserReviewState::Pending,
        source_id: None,
    };
    draft.user_acceptance = UserAcceptanceFact {
        state: UserAcceptanceState::Accepted,
        source_id: Some(user.id),
    };
    draft.known_limits = vec!["No automatic checkpoint selection".to_owned()];
    draft.open_questions = vec![QuestionReference {
        question_id: open.id,
        revision: open.revision,
    }];
    let checkpoint = store
        .record_checkpoint(operation(34), project.id, draft)?
        .value;
    assert_eq!(checkpoint.work_state, WorkState::Completed);
    assert_eq!(checkpoint.verification[0].state, VerificationState::Failed);
    assert_eq!(checkpoint.user_review.state, UserReviewState::Pending);
    assert_eq!(
        checkpoint.user_acceptance.state,
        UserAcceptanceState::Accepted
    );
    assert_eq!(checkpoint.user_acceptance.source_id, Some(user.id));
    assert_eq!(checkpoint.changed_source_basis, vec![repository.id]);
    assert_eq!(checkpoint.applied_decisions, vec![decision.id]);
    assert_eq!(checkpoint.open_questions[0].question_id, open.id);
    assert_eq!(store.get_checkpoint(project.id, checkpoint.id)?, checkpoint);
    Ok(())
}

#[test]
fn records_pause_and_handoff_without_attributing_unsupplied_changes(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut store = store_with_ids(&root.path().join("context.sqlite3"), &[1, 2, 3, 4, 5, 6, 7])?;
    let project = store.create_project(operation(40), "Boundaries")?.value;
    let (repository, _, _, _) = record_sources(&mut store, &project)?;
    let pause = store
        .record_checkpoint(
            operation(41),
            project.id,
            checkpoint_draft(CheckpointKind::Pause, WorkState::Paused, repository.id),
        )?
        .value;
    assert!(pause.changed_paths.is_empty());
    assert!(pause.changed_source_basis.is_empty());

    let mut handoff = checkpoint_draft(
        CheckpointKind::Handoff,
        WorkState::InProgress,
        repository.id,
    );
    handoff.handoff_to = Some("next agent session".to_owned());
    let handoff = store
        .record_checkpoint(operation(42), project.id, handoff)?
        .value;
    assert_eq!(handoff.handoff_to.as_deref(), Some("next agent session"));
    assert!(handoff.changed_paths.is_empty());
    Ok(())
}

#[test]
fn rejects_empty_read_only_completion_and_unexecuted_verification_claim(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut store = store_with_ids(&root.path().join("context.sqlite3"), &[1, 2, 3, 4, 5])?;
    let project = store.create_project(operation(50), "Reject")?.value;
    let (repository, _, _, _) = record_sources(&mut store, &project)?;
    let empty = checkpoint_draft(
        CheckpointKind::Completion,
        WorkState::Completed,
        repository.id,
    );
    assert_eq!(
        store
            .record_checkpoint(operation(51), project.id, empty)
            .err()
            .ok_or("expected empty completion rejection")?
            .kind(),
        ErrorKind::InvalidInput
    );
    let mut false_claim = checkpoint_draft(CheckpointKind::Pause, WorkState::Paused, repository.id);
    false_claim.verification = vec![VerificationFact {
        state: VerificationState::Passed,
        source_id: None,
        outcome: Some("not actually run".to_owned()),
    }];
    assert_eq!(
        store
            .record_checkpoint(operation(52), project.id, false_claim)
            .err()
            .ok_or("expected verification provenance rejection")?
            .kind(),
        ErrorKind::InvalidInput
    );
    Ok(())
}

#[test]
fn context_and_checkpoint_replay_consistently_after_restart(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let path = root.path().join("context.sqlite3");
    let mut store = store_with_ids(&path, &[1, 2, 3, 4, 5, 6, 7])?;
    let project = store.create_project(operation(60), "Replay")?.value;
    let (repository, _, _, _) = record_sources(&mut store, &project)?;
    let item_draft = context_draft(
        ContextItemRole::Fact,
        StatementProvenanceRole::Observed,
        PrincipalKind::Agent,
        repository.id,
    );
    let item = store.record_context_item(operation(61), project.id, item_draft.clone())?;
    let mut checkpoint_draft = checkpoint_draft(
        CheckpointKind::Completion,
        WorkState::Completed,
        repository.id,
    );
    checkpoint_draft.state_change = Some("Recorded durable state".to_owned());
    let checkpoint =
        store.record_checkpoint(operation(62), project.id, checkpoint_draft.clone())?;
    drop(store);

    let mut reopened = store_with_ids(&path, &[])?;
    let item_replay =
        reopened.record_context_item(operation(61), project.id, item_draft.clone())?;
    let checkpoint_replay =
        reopened.record_checkpoint(operation(62), project.id, checkpoint_draft.clone())?;
    assert!(item_replay.replayed);
    assert!(checkpoint_replay.replayed);
    assert_eq!(item_replay.value, item.value);
    assert_eq!(checkpoint_replay.value, checkpoint.value);
    let mut changed = checkpoint_draft;
    changed.next_step = "Changed operation input".to_owned();
    assert_eq!(
        reopened
            .record_checkpoint(operation(62), project.id, changed)
            .err()
            .ok_or("expected operation mismatch")?
            .kind(),
        ErrorKind::DomainConflict
    );
    Ok(())
}
