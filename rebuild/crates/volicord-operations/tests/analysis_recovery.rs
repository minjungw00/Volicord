use rusqlite::Connection;
use std::{fs, path::PathBuf};
use tempfile::TempDir;
use volicord_context::{
    AgentRecommendation, ApplicabilityScope, Availability, CanonicalReadBasis, CanonicalRecordKind,
    CheckpointDraft, CheckpointKind, ContextItemCorrectionDraft, ContextItemDraft, ContextItemRole,
    CorrectionKind, DecisionChoice, DecisionSupersessionDraft, ExplicitQuestionResponse,
    NonUserQuestionOutcome, OperationId, Principal, PrincipalKind, QuestionAlternative,
    QuestionDraft, QuestionMateriality, QuestionReference, QuestionResearchState,
    QuestionResponseDraft, SourceDraft, SourceId, SourcePayload, StatementProvenanceRole, Store,
    UserAcceptanceFact, UserAcceptanceState, UserReviewFact, UserReviewState, UserTurnSource,
    VerificationFact, VerificationState, WorkState,
};
use volicord_operations::{
    HealthIssueKind, HealthState, LocalOperations, OperationState, RepairKind, RuntimeLayout,
};

struct Fixture {
    _temporary: TempDir,
    operations: LocalOperations,
    repository: PathBuf,
}

fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let repository = temporary.path().join("repository");
    fs::create_dir_all(repository.join("src"))?;
    fs::write(
        repository.join("src/main.py"),
        "def answer():\n    return 42\n",
    )?;
    let operations = LocalOperations::new(RuntimeLayout::new(temporary.path().join("runtime"))?);
    Ok(Fixture {
        _temporary: temporary,
        operations,
        repository,
    })
}

fn export_bytes(
    operations: &LocalOperations,
    project: volicord_context::ProjectId,
    destination: &std::path::Path,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    operations.export_bundle(project, destination)?;
    Ok(fs::read(destination)?)
}

fn question_draft(repository_source: SourceId, prompt: &str) -> QuestionDraft {
    QuestionDraft {
        expected_project_revision: 1,
        prompt_basis: prompt.into(),
        source_basis: vec![repository_source],
        dependencies: Vec::new(),
        alternatives: vec![QuestionAlternative {
            key: "keep".into(),
            label: "Keep".into(),
            consequence: "Preserve the current meaning".into(),
        }],
        recommendation: AgentRecommendation {
            alternative_key: Some("keep".into()),
            rationale: "The user-owned meaning remains applicable".into(),
            source_basis: vec![repository_source],
        },
        trade_offs: vec!["Historical repository provenance remains inspectable".into()],
        uncertainty: Vec::new(),
        material_scope: vec!["analysis recovery".into()],
        materiality: QuestionMateriality::Material,
        presentation_order: 0,
        why_it_matters_now: "Recovery must preserve user judgment".into(),
        established_facts: Vec::new(),
        assumptions: Vec::new(),
        known_limits: Vec::new(),
        what_the_answer_unlocks: vec!["safe rebuild".into()],
        allowed_non_choice_dispositions: NonUserQuestionOutcome::ALL.to_vec(),
        research_state: QuestionResearchState::ReadyToAsk,
    }
}

fn add_user_owned_meaning(
    operations: &LocalOperations,
    project: volicord_context::ProjectId,
    repository_source: SourceId,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut store = Store::open(operations.layout().canonical_store())?;
    let project_revision = store.get_project(project)?.revision;
    let user = store
        .record_source(
            OperationId::from_bytes([71; 16]),
            project,
            SourceDraft {
                expected_project_revision: project_revision,
                payload: SourcePayload::CurrentHostUserTurn {
                    host: "test-host".into(),
                    session: "repair-session".into(),
                    turn: "remember the corrected constraint".into(),
                },
                actor: Principal {
                    kind: PrincipalKind::User,
                    identity: "current-user".into(),
                },
                observer: Some(Principal {
                    kind: PrincipalKind::Agent,
                    identity: "operations-test".into(),
                }),
                availability: Availability::Available,
            },
        )?
        .value;
    let item = store
        .record_context_item(
            OperationId::from_bytes([72; 16]),
            project,
            ContextItemDraft {
                expected_project_revision: project_revision,
                role: ContextItemRole::Constraint,
                statement: "keep derive state local".into(),
                provenance_role: StatementProvenanceRole::UserStatement,
                author: user.actor.clone(),
                source_basis: vec![user.id],
                applicability: ApplicabilityScope::default(),
            },
        )?
        .value;
    store.correct_context_item(
        OperationId::from_bytes([73; 16]),
        project,
        item.id,
        ContextItemCorrectionDraft {
            expected_revision: item.revision,
            corrected_statement: "keep derived state local".into(),
            kind: CorrectionKind::Typography,
            user_authorization_source_id: user.id,
        },
    )?;

    let answered = store
        .create_question(
            OperationId::from_bytes([74; 16]),
            project,
            question_draft(repository_source, "Keep recovery provenance truthful?"),
        )?
        .value;
    let original_decision = store
        .record_question_response(
            OperationId::from_bytes([75; 16]),
            project,
            QuestionResponseDraft {
                expected_project_revision: project_revision,
                question_id: answered.id,
                question_revision: answered.revision,
                user_turn_source: UserTurnSource::Create(SourceDraft {
                    expected_project_revision: project_revision,
                    payload: SourcePayload::CurrentHostUserTurn {
                        host: "test-host".into(),
                        session: "repair-session".into(),
                        turn: "Keep repository observations attributable".into(),
                    },
                    actor: Principal {
                        kind: PrincipalKind::User,
                        identity: "current-user".into(),
                    },
                    observer: Some(Principal {
                        kind: PrincipalKind::Agent,
                        identity: "operations-test".into(),
                    }),
                    availability: Availability::Available,
                }),
                displayed_alternative_keys: vec!["keep".into()],
                displayed_recommendation_key: Some("keep".into()),
                response: ExplicitQuestionResponse::Choice {
                    alternative_key: "keep".into(),
                    user_rationale: Some("Repository observations must remain attributable".into()),
                },
                applicability: ApplicabilityScope::default(),
                assumptions: Vec::new(),
                revisit_triggers: Vec::new(),
            },
        )?
        .value
        .decision
        .ok_or("expected user Decision")?;
    let current_decision = store
        .supersede_decision(
            OperationId::from_bytes([76; 16]),
            project,
            DecisionSupersessionDraft {
                expected_project_revision: project_revision,
                previous_decision_id: original_decision.id,
                user_turn_source: UserTurnSource::Create(SourceDraft {
                    expected_project_revision: project_revision,
                    payload: SourcePayload::CurrentHostUserTurn {
                        host: "test-host".into(),
                        session: "repair-session".into(),
                        turn: "Preserve provenance across every rebuild".into(),
                    },
                    actor: Principal {
                        kind: PrincipalKind::User,
                        identity: "current-user".into(),
                    },
                    observer: Some(Principal {
                        kind: PrincipalKind::Agent,
                        identity: "operations-test".into(),
                    }),
                    availability: Availability::Available,
                }),
                choice: DecisionChoice::Alternative {
                    alternative_key: "keep".into(),
                },
                user_rationale: Some("Preserve provenance across every rebuild".into()),
                applicability: ApplicabilityScope::default(),
                assumptions: Vec::new(),
                revisit_triggers: Vec::new(),
            },
        )?
        .value;
    let open = store
        .create_question(
            OperationId::from_bytes([77; 16]),
            project,
            question_draft(repository_source, "Which analysis scope comes next?"),
        )?
        .value;
    store.record_checkpoint(
        OperationId::from_bytes([78; 16]),
        project,
        CheckpointDraft {
            expected_project_revision: project_revision,
            kind: CheckpointKind::Pause,
            goal: "Protect analysis recovery".into(),
            work_state: WorkState::Paused,
            state_change: Some("User-owned recovery meaning recorded".into()),
            source_basis: vec![repository_source],
            changed_source_basis: Vec::new(),
            changed_paths: Vec::new(),
            applied_decisions: vec![current_decision.id],
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
            known_limits: Vec::new(),
            non_goals: Vec::new(),
            open_questions: vec![QuestionReference {
                question_id: open.id,
                revision: open.revision,
            }],
            next_step: "Repair derived analysis".into(),
            handoff_to: None,
        },
    )?;
    let disposable = store
        .record_source(
            OperationId::from_bytes([79; 16]),
            project,
            SourceDraft {
                expected_project_revision: project_revision,
                payload: SourcePayload::Url {
                    url: "https://example.invalid/forgotten".into(),
                },
                actor: Principal {
                    kind: PrincipalKind::Agent,
                    identity: "operations-test".into(),
                },
                observer: None,
                availability: Availability::Available,
            },
        )?
        .value;
    store.forget_source(
        OperationId::from_bytes([80; 16]),
        project,
        disposable.id,
        user.id,
    )?;
    Ok(())
}

fn assert_user_owned_meaning_preserved(before: &CanonicalReadBasis, after: &CanonicalReadBasis) {
    assert_eq!(before.project, after.project);
    assert_eq!(before.active_questions, after.active_questions);
    assert_eq!(
        before.terminal_question_history,
        after.terminal_question_history
    );
    assert_eq!(before.active_decisions, after.active_decisions);
    assert_eq!(before.superseded_decisions, after.superseded_decisions);
    assert_eq!(before.context_items, after.context_items);
    assert_eq!(before.latest_checkpoint, after.latest_checkpoint);
    assert_eq!(before.checkpoint_history, after.checkpoint_history);
    assert_eq!(before.forgotten, after.forgotten);
    assert_eq!(
        before.forgotten_checkpoint_sources,
        after.forgotten_checkpoint_sources
    );
    assert_eq!(before.bundle_merges, after.bundle_merges);
    assert_eq!(
        before
            .sources
            .iter()
            .filter(|source| !matches!(
                source.source.payload,
                SourcePayload::RepositorySnapshot { .. }
            ))
            .collect::<Vec<_>>(),
        after
            .sources
            .iter()
            .filter(|source| !matches!(
                source.source.payload,
                SourcePayload::RepositorySnapshot { .. }
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        before
            .revisions
            .iter()
            .filter(|revision| revision.record_kind != CanonicalRecordKind::Source)
            .collect::<Vec<_>>(),
        after
            .revisions
            .iter()
            .filter(|revision| revision.record_kind != CanonicalRecordKind::Source)
            .collect::<Vec<_>>()
    );
}

#[test]
fn corrupt_analysis_repair_observes_current_repository_and_preserves_user_meaning(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let initialized = fixture
        .operations
        .initialize_project("Repair fixture", Some(&fixture.repository))?;
    let analyzed = fixture
        .operations
        .analyze(initialized.project.id, Vec::new())?;
    let first = analyzed.value.ok_or("analysis result missing")?;
    let stored = first.stored_at.clone();
    add_user_owned_meaning(
        &fixture.operations,
        initialized.project.id,
        first.repository.repository_source.identity(),
    )?;
    let before_basis = fixture.operations.canonical_basis(initialized.project.id)?;
    let historical_repository_source = before_basis
        .sources
        .iter()
        .find(|source| source.source.id == first.repository.repository_source.identity())
        .cloned()
        .ok_or("initial repository Source missing from canonical history")?;

    fs::write(
        fixture.repository.join("src/repair-current.py"),
        "REPAIRED_CURRENT = True\n",
    )?;
    fs::write(&stored, b"{ corrupt derived bytes")?;
    let degraded = fixture.operations.health(Some(initialized.project.id));
    assert_eq!(degraded.state, HealthState::Degraded);
    assert!(degraded.issues.iter().any(|issue| {
        issue.kind == HealthIssueKind::Corrupt
            && issue.scope == format!("derived_analysis:{}", initialized.project.id)
    }));

    let repaired =
        fixture
            .operations
            .repair(initialized.project.id, "derived-analysis", Vec::new())?;
    assert_eq!(repaired.kind, RepairKind::DerivedAnalysisRepair);
    assert_eq!(repaired.discarded_entries, 1);
    assert!(repaired.diagnosis.contains("corrupt"));
    assert!(matches!(
        repaired.operation.state,
        OperationState::Succeeded | OperationState::Partial
    ));
    let repaired_value = repaired
        .operation
        .value
        .ok_or("repaired analysis missing")?;
    assert_ne!(
        repaired_value.repository.identity,
        first.repository.identity
    );
    assert_ne!(
        repaired_value.repository.repository_source,
        first.repository.repository_source
    );
    assert_eq!(
        repaired_value.repository.repository_source,
        repaired_value.analysis.repository_source
    );
    assert!(repaired_value
        .analysis
        .inventory
        .entries
        .iter()
        .any(|entry| entry.area.path == "src/repair-current.py"));
    assert!(!fixture
        .operations
        .recall(initialized.project.id)?
        .snapshots
        .is_empty());
    assert_eq!(
        fixture
            .operations
            .health(Some(initialized.project.id))
            .state,
        HealthState::Healthy
    );

    let after_basis = fixture.operations.canonical_basis(initialized.project.id)?;
    assert_user_owned_meaning_preserved(&before_basis, &after_basis);
    assert_eq!(
        after_basis
            .sources
            .iter()
            .find(|source| source.source.id == first.repository.repository_source.identity())
            .ok_or("historical repository Source disappeared during repair")?,
        &historical_repository_source
    );
    assert!(after_basis.sources.iter().any(|source| {
        source.source.id == repaired_value.repository.repository_source.identity()
            && matches!(
                source.source.payload,
                SourcePayload::RepositorySnapshot { .. }
            )
    }));
    let corrected = after_basis
        .context_items
        .iter()
        .find(|item| item.statement == "keep derived state local")
        .ok_or("corrected Context Item was not preserved")?;
    assert_eq!(corrected.revision, 2);
    Ok(())
}

#[test]
fn forced_reindex_discards_prior_snapshot_and_observes_current_repository(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let initialized = fixture
        .operations
        .initialize_project("Reindex fixture", Some(&fixture.repository))?;
    let first = fixture
        .operations
        .analyze(initialized.project.id, Vec::new())?;
    let first = first.value.ok_or("first analysis missing")?;
    add_user_owned_meaning(
        &fixture.operations,
        initialized.project.id,
        first.repository.repository_source.identity(),
    )?;
    let before_basis = fixture.operations.canonical_basis(initialized.project.id)?;
    let historical_repository_source = before_basis
        .sources
        .iter()
        .find(|source| source.source.id == first.repository.repository_source.identity())
        .cloned()
        .ok_or("initial repository Source missing from canonical history")?;
    let before_bundle = export_bytes(
        &fixture.operations,
        initialized.project.id,
        &fixture._temporary.path().join("reindex-before.json"),
    )?;
    fs::write(
        fixture.repository.join("src/current.py"),
        "CURRENT = True\n",
    )?;

    let rebuilt = fixture
        .operations
        .reindex(initialized.project.id, Vec::new())?;
    assert_eq!(rebuilt.kind, RepairKind::DerivedRebuild);
    assert_eq!(rebuilt.discarded_entries, 1);
    let value = rebuilt.operation.value.ok_or("rebuilt analysis missing")?;
    assert_ne!(value.repository.identity, first.repository.identity);
    assert_ne!(
        value.repository.repository_source,
        first.repository.repository_source
    );
    assert_eq!(
        value.repository.repository_source,
        value.analysis.repository_source
    );
    assert!(!first.stored_at.exists());
    assert!(value.stored_at.exists());
    assert!(value
        .analysis
        .inventory
        .entries
        .iter()
        .any(|entry| entry.area.path == "src/current.py"));
    assert_eq!(
        fs::read_dir(
            fixture
                .operations
                .layout()
                .analysis_project_dir(initialized.project.id)
        )?
        .count(),
        1
    );
    let after_bundle = export_bytes(
        &fixture.operations,
        initialized.project.id,
        &fixture._temporary.path().join("reindex-after.json"),
    )?;
    assert_ne!(before_bundle, after_bundle);
    let canonical = fixture.operations.canonical_basis(initialized.project.id)?;
    assert_user_owned_meaning_preserved(&before_basis, &canonical);
    assert_eq!(
        canonical
            .sources
            .iter()
            .find(|source| source.source.id == first.repository.repository_source.identity())
            .ok_or("historical repository Source disappeared during reindex")?,
        &historical_repository_source
    );
    let current_source = canonical
        .sources
        .iter()
        .find(|source| source.source.id == value.repository.repository_source.identity())
        .ok_or("fresh repository Source missing from canonical basis")?;
    assert!(matches!(
        current_source.source.payload,
        SourcePayload::RepositorySnapshot { .. }
    ));
    assert_eq!(current_source.source.actor.kind, PrincipalKind::Repository);
    Ok(())
}

#[test]
fn project_owned_analysis_recovery_does_not_touch_another_project(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let other_repository = fixture._temporary.path().join("other-repository");
    fs::create_dir_all(&other_repository)?;
    fs::write(other_repository.join("main.go"), "package main\n")?;
    let first = fixture
        .operations
        .initialize_project("First", Some(&fixture.repository))?;
    let second = fixture
        .operations
        .initialize_project("Second", Some(&other_repository))?;
    let first_analysis = fixture
        .operations
        .analyze(first.project.id, Vec::new())?
        .value
        .ok_or("first analysis missing")?;
    let second_analysis = fixture
        .operations
        .analyze(second.project.id, Vec::new())?
        .value
        .ok_or("second analysis missing")?;
    let second_path = second_analysis.stored_at.clone();
    let second_bytes = fs::read(&second_path)?;
    let second_canonical = fixture.operations.canonical_basis(second.project.id)?;

    fs::write(&first_analysis.stored_at, b"not-json")?;
    assert_eq!(
        fixture.operations.health(Some(second.project.id)).state,
        HealthState::Healthy
    );
    assert!(!fixture
        .operations
        .recall(second.project.id)?
        .snapshots
        .is_empty());
    fixture
        .operations
        .repair(first.project.id, "derived-analysis", Vec::new())?;

    assert_eq!(fs::read(&second_path)?, second_bytes);
    assert_eq!(
        fixture.operations.canonical_basis(second.project.id)?,
        second_canonical
    );
    assert_eq!(
        fixture.operations.health(Some(second.project.id)).state,
        HealthState::Healthy
    );
    assert!(!fixture
        .operations
        .recall(second.project.id)?
        .snapshots
        .is_empty());
    Ok(())
}

#[test]
fn failed_repository_source_recording_does_not_publish_rebuilt_analysis(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let initialized = fixture
        .operations
        .initialize_project("Observation failure", Some(&fixture.repository))?;
    let first = fixture
        .operations
        .analyze(initialized.project.id, Vec::new())?
        .value
        .ok_or("initial analysis missing")?;
    let before_bytes = fs::read(&first.stored_at)?;
    let connection = Connection::open(fixture.operations.layout().canonical_store())?;
    connection.execute_batch(
        "CREATE TRIGGER reject_repository_observation BEFORE INSERT ON sources
         BEGIN SELECT RAISE(ABORT, 'repository observation fault'); END;",
    )?;
    drop(connection);
    fs::write(
        fixture.repository.join("src/unpublished.py"),
        "UNPUBLISHED = True\n",
    )?;

    let error = fixture
        .operations
        .reindex(initialized.project.id, Vec::new())
        .expect_err("reindex must fail before derived publication");
    assert!(error
        .message()
        .contains("cannot record repository observation Source"));
    assert_eq!(fs::read(&first.stored_at)?, before_bytes);
    assert_eq!(
        fs::read_dir(
            fixture
                .operations
                .layout()
                .analysis_project_dir(initialized.project.id)
        )?
        .count(),
        1
    );
    let canonical = fixture.operations.canonical_basis(initialized.project.id)?;
    assert_eq!(
        canonical
            .sources
            .iter()
            .filter(|source| matches!(
                source.source.payload,
                SourcePayload::RepositorySnapshot { .. }
            ))
            .count(),
        1
    );
    Ok(())
}
