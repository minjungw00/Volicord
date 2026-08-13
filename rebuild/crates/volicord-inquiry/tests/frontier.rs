use tempfile::tempdir;
use volicord_context::{
    AgentRecommendation, Availability, CanonicalReadOptions, DeterministicIdGenerator, FixedClock,
    NonUserQuestionOutcome, OperationId, Principal, PrincipalKind, QuestionAlternative,
    QuestionDependency, QuestionDispositionDraft, QuestionDraft, QuestionId, QuestionMateriality,
    QuestionResearchState, QuestionTerminalOutcome, SourceDraft, SourcePayload, Store,
    TimestampMicros,
};
use volicord_inquiry::{
    compute_frontier, resolve_question_by_research, FrontierDiagnosticKind, InquiryScope,
};

fn operation(value: u8) -> OperationId {
    OperationId::from_bytes([value; 16])
}

fn question_draft(
    source: volicord_context::SourceId,
    prompt: &str,
    order: u64,
    dependencies: Vec<QuestionDependency>,
) -> QuestionDraft {
    QuestionDraft {
        expected_project_revision: 1,
        prompt_basis: prompt.to_owned(),
        source_basis: vec![source],
        dependencies,
        alternatives: vec![QuestionAlternative {
            key: "continue".to_owned(),
            label: "Continue".to_owned(),
            consequence: "Open the dependent branch".to_owned(),
        }],
        recommendation: AgentRecommendation {
            alternative_key: Some("continue".to_owned()),
            rationale: "the source basis supports continuing".to_owned(),
            source_basis: vec![source],
        },
        trade_offs: vec!["later revision may change the basis".to_owned()],
        uncertainty: vec!["runtime-only behavior is unknown".to_owned()],
        material_scope: vec!["storage".to_owned()],
        presentation_order: order,
        why_it_matters_now: "the dependent implementation is blocked".to_owned(),
        what_the_answer_unlocks: vec!["dependent implementation".to_owned()],
        materiality: QuestionMateriality::Material,
        established_facts: vec![],
        assumptions: vec![],
        known_limits: vec![],
        allowed_non_choice_dispositions: NonUserQuestionOutcome::ALL.to_vec(),
        research_state: QuestionResearchState::ReadyToAsk,
    }
}

#[test]
fn frontier_requires_the_exact_positive_outcome_and_source_basis(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut store = Store::open_with(
        root.path().join("context.sqlite3"),
        DeterministicIdGenerator::new((1_u8..=8).map(|value| [value; 16])),
        FixedClock::new(TimestampMicros::from_unix_micros(1_000)),
    )?;
    let project = store.create_project(operation(1), "Frontier")?.value;
    let source = store
        .record_source(
            operation(2),
            project.id,
            SourceDraft {
                expected_project_revision: 1,
                payload: SourcePayload::RepositorySnapshot {
                    revision: "commit-a".to_owned(),
                },
                actor: Principal {
                    kind: PrincipalKind::Repository,
                    identity: "repository".to_owned(),
                },
                observer: Some(Principal {
                    kind: PrincipalKind::Agent,
                    identity: "codex".to_owned(),
                }),
                availability: Availability::Available,
            },
        )?
        .value;
    let prerequisite = store
        .create_question(
            operation(3),
            project.id,
            question_draft(source.id, "Establish the repository fact", 20, vec![]),
        )?
        .value;
    let dependent = store
        .create_question(
            operation(4),
            project.id,
            question_draft(
                source.id,
                "Choose the implementation after research",
                10,
                vec![QuestionDependency {
                    question_id: prerequisite.id,
                    required_revision: prerequisite.revision,
                    required_outcome: QuestionTerminalOutcome::ResolvedByResearch,
                    required_source_basis: vec![source.id],
                    blocked_outcomes: vec![QuestionTerminalOutcome::Deferred],
                    superseding_outcomes: vec![QuestionTerminalOutcome::OutOfScope],
                    assessment_source_basis: vec![source.id],
                }],
            ),
        )?
        .value;
    let scope = InquiryScope {
        project_id: project.id,
        material_scope: vec!["storage".to_owned()],
    };
    let before = store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
    let initial = compute_frontier(&before, &scope);
    assert_eq!(initial.questions[0].question_id, prerequisite.id);
    assert!(initial.diagnostics.iter().any(|diagnostic| {
        diagnostic.question_id == dependent.id
            && diagnostic.kind == FrontierDiagnosticKind::UnsatisfiedOutcome
    }));

    resolve_question_by_research(
        &mut store,
        &before,
        operation(5),
        project.id,
        QuestionDispositionDraft {
            expected_project_revision: 1,
            question_id: prerequisite.id,
            question_revision: prerequisite.revision,
            outcome: NonUserQuestionOutcome::ResolvedByResearch,
            source_basis: vec![source.id],
            reason: "repository snapshot establishes the fact".to_owned(),
            replacement_question_id: None,
            revisit_basis: vec![],
            actor: Principal {
                kind: PrincipalKind::Repository,
                identity: "local-observer".to_owned(),
            },
        },
    )?;
    let after = store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
    let repeated = after.clone();
    let current = compute_frontier(&after, &scope);
    assert_eq!(current.questions.len(), 1);
    assert_eq!(current.questions[0].question_id, dependent.id);
    assert_eq!(current.questions[0].displayed_revision, dependent.revision);
    assert_eq!(
        current.questions[0].why_it_matters_now,
        dependent.why_it_matters_now
    );
    assert!(current.diagnostics.is_empty());
    assert_eq!(after, repeated);
    assert!(after.active_decisions.is_empty());
    Ok(())
}

#[test]
fn frontier_distinguishes_blocking_and_superseding_outcomes(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut store = Store::open_with(
        root.path().join("context.sqlite3"),
        DeterministicIdGenerator::new((1_u8..=10).map(|value| [value; 16])),
        FixedClock::new(TimestampMicros::from_unix_micros(1_000)),
    )?;
    let project = store
        .create_project(operation(40), "Outcome branches")?
        .value;
    let source = store
        .record_source(
            operation(41),
            project.id,
            SourceDraft {
                expected_project_revision: 1,
                payload: SourcePayload::RepositorySnapshot {
                    revision: "commit-outcomes".to_owned(),
                },
                actor: Principal {
                    kind: PrincipalKind::Repository,
                    identity: "repository".to_owned(),
                },
                observer: None,
                availability: Availability::Available,
            },
        )?
        .value;
    let blocker = store
        .create_question(
            operation(42),
            project.id,
            question_draft(source.id, "Blocking prerequisite", 0, vec![]),
        )?
        .value;
    let superseder = store
        .create_question(
            operation(43),
            project.id,
            question_draft(source.id, "Superseding prerequisite", 1, vec![]),
        )?
        .value;
    let blocked_branch = store
        .create_question(
            operation(44),
            project.id,
            question_draft(
                source.id,
                "Blocked branch",
                2,
                vec![QuestionDependency {
                    question_id: blocker.id,
                    required_revision: blocker.revision,
                    required_outcome: QuestionTerminalOutcome::ResolvedByResearch,
                    required_source_basis: vec![source.id],
                    blocked_outcomes: vec![QuestionTerminalOutcome::Deferred],
                    superseding_outcomes: vec![],
                    assessment_source_basis: vec![source.id],
                }],
            ),
        )?
        .value;
    let superseded_branch = store
        .create_question(
            operation(45),
            project.id,
            question_draft(
                source.id,
                "Superseded branch",
                3,
                vec![QuestionDependency {
                    question_id: superseder.id,
                    required_revision: superseder.revision,
                    required_outcome: QuestionTerminalOutcome::ResolvedByResearch,
                    required_source_basis: vec![source.id],
                    blocked_outcomes: vec![],
                    superseding_outcomes: vec![QuestionTerminalOutcome::OutOfScope],
                    assessment_source_basis: vec![source.id],
                }],
            ),
        )?
        .value;
    store.dispose_question(
        operation(46),
        project.id,
        QuestionDispositionDraft {
            expected_project_revision: 1,
            question_id: blocker.id,
            question_revision: blocker.revision,
            outcome: NonUserQuestionOutcome::Deferred,
            source_basis: vec![source.id],
            reason: "wait for the repository boundary".to_owned(),
            replacement_question_id: None,
            revisit_basis: vec!["repository boundary changes".to_owned()],
            actor: Principal {
                kind: PrincipalKind::Agent,
                identity: "inquiry".to_owned(),
            },
        },
    )?;
    store.dispose_question(
        operation(47),
        project.id,
        QuestionDispositionDraft {
            expected_project_revision: 1,
            question_id: superseder.id,
            question_revision: superseder.revision,
            outcome: NonUserQuestionOutcome::OutOfScope,
            source_basis: vec![source.id],
            reason: "the prerequisite is outside this Project".to_owned(),
            replacement_question_id: None,
            revisit_basis: vec![],
            actor: Principal {
                kind: PrincipalKind::Agent,
                identity: "inquiry".to_owned(),
            },
        },
    )?;

    let basis = store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
    let frontier = compute_frontier(
        &basis,
        &InquiryScope {
            project_id: project.id,
            material_scope: vec!["storage".to_owned()],
        },
    );
    assert!(frontier.questions.is_empty());
    assert!(frontier.diagnostics.iter().any(|diagnostic| {
        diagnostic.question_id == blocked_branch.id
            && diagnostic.kind == FrontierDiagnosticKind::BlockedByPrerequisite
    }));
    assert!(frontier.diagnostics.iter().any(|diagnostic| {
        diagnostic.question_id == superseded_branch.id
            && diagnostic.kind == FrontierDiagnosticKind::SupersededByPrerequisite
    }));
    Ok(())
}

#[test]
fn frontier_reports_cycles_missing_revision_and_basis_deterministically(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut store = Store::open_with(
        root.path().join("context.sqlite3"),
        DeterministicIdGenerator::new((1_u8..=8).map(|value| [value; 16])),
        FixedClock::new(TimestampMicros::from_unix_micros(1_000)),
    )?;
    let project = store.create_project(operation(10), "Diagnostics")?.value;
    let source = store
        .record_source(
            operation(11),
            project.id,
            SourceDraft {
                expected_project_revision: 1,
                payload: SourcePayload::File {
                    locator: "src/lib.rs".to_owned(),
                    snapshot: "commit-a".to_owned(),
                },
                actor: Principal {
                    kind: PrincipalKind::Repository,
                    identity: "repository".to_owned(),
                },
                observer: None,
                availability: Availability::Available,
            },
        )?
        .value;
    let first = store
        .create_question(
            operation(12),
            project.id,
            question_draft(source.id, "First", 2, vec![]),
        )?
        .value;
    let second = store
        .create_question(
            operation(13),
            project.id,
            question_draft(source.id, "Second", 1, vec![]),
        )?
        .value;
    let third = store
        .create_question(
            operation(14),
            project.id,
            question_draft(source.id, "Third", 1, vec![]),
        )?
        .value;
    let scope = InquiryScope {
        project_id: project.id,
        material_scope: vec![],
    };
    let ordered = compute_frontier(
        &store.read_canonical_basis(project.id, CanonicalReadOptions::default())?,
        &scope,
    );
    assert_eq!(
        ordered
            .questions
            .iter()
            .map(|question| question.question_id)
            .collect::<Vec<_>>(),
        vec![second.id, third.id, first.id]
    );
    let mut basis = store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
    let invalid_source = volicord_context::SourceId::from_bytes([99; 16]);
    basis.active_questions[0].dependencies = vec![QuestionDependency {
        question_id: second.id,
        required_revision: 99,
        required_outcome: QuestionTerminalOutcome::Answered,
        required_source_basis: vec![invalid_source],
        blocked_outcomes: vec![],
        superseding_outcomes: vec![],
        assessment_source_basis: vec![source.id],
    }];
    basis.active_questions[1].dependencies = vec![QuestionDependency {
        question_id: first.id,
        required_revision: first.revision,
        required_outcome: QuestionTerminalOutcome::Answered,
        required_source_basis: vec![],
        blocked_outcomes: vec![],
        superseding_outcomes: vec![],
        assessment_source_basis: vec![source.id],
    }];
    basis.active_questions[2].dependencies = vec![QuestionDependency {
        question_id: third.id,
        required_revision: third.revision,
        required_outcome: QuestionTerminalOutcome::Answered,
        required_source_basis: vec![],
        blocked_outcomes: vec![],
        superseding_outcomes: vec![],
        assessment_source_basis: vec![source.id],
    }];
    let first_read = compute_frontier(&basis, &scope);
    let second_read = compute_frontier(&basis, &scope);
    assert_eq!(first_read, second_read);
    assert!(first_read.questions.is_empty());
    assert_eq!(
        first_read
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.kind == FrontierDiagnosticKind::DependencyCycle)
            .count(),
        3
    );

    let mut missing = store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
    missing.active_questions[0].dependencies = vec![QuestionDependency {
        question_id: QuestionId::from_bytes([77; 16]),
        required_revision: 1,
        required_outcome: QuestionTerminalOutcome::Answered,
        required_source_basis: vec![],
        blocked_outcomes: vec![],
        superseding_outcomes: vec![],
        assessment_source_basis: vec![source.id],
    }];
    let missing_read = compute_frontier(&missing, &scope);
    assert!(missing_read
        .diagnostics
        .iter()
        .any(|diagnostic| { diagnostic.kind == FrontierDiagnosticKind::MissingPrerequisite }));

    let mut invalid_revision =
        store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
    invalid_revision.active_questions[0].dependencies = vec![QuestionDependency {
        question_id: second.id,
        required_revision: 99,
        required_outcome: QuestionTerminalOutcome::Answered,
        required_source_basis: vec![],
        blocked_outcomes: vec![],
        superseding_outcomes: vec![],
        assessment_source_basis: vec![source.id],
    }];
    let invalid_revision_read = compute_frontier(&invalid_revision, &scope);
    assert!(invalid_revision_read.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == FrontierDiagnosticKind::InvalidDependencyRevision
    }));

    let mut invalid_basis =
        store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
    invalid_basis.active_questions[0].dependencies = vec![QuestionDependency {
        question_id: second.id,
        required_revision: second.revision,
        required_outcome: QuestionTerminalOutcome::Answered,
        required_source_basis: vec![invalid_source],
        blocked_outcomes: vec![],
        superseding_outcomes: vec![],
        assessment_source_basis: vec![source.id],
    }];
    let invalid_basis_read = compute_frontier(&invalid_basis, &scope);
    assert!(invalid_basis_read
        .diagnostics
        .iter()
        .any(|diagnostic| { diagnostic.kind == FrontierDiagnosticKind::InvalidDependencyBasis }));
    Ok(())
}
