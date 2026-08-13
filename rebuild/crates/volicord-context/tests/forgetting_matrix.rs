use rusqlite::Connection;
use std::fs;
use std::path::Path;
use tempfile::tempdir;
use volicord_context::{
    AgentRecommendation, ApplicabilityScope, Availability, BundleConflictClass, BundleMergeStatus,
    CanonicalReadOptions, CanonicalRecordId, CanonicalRecordKind, CanonicalRelationKind,
    CheckpointDraft, CheckpointKind, ContextItemCorrectionDraft, ContextItemDraft, ContextItemRole,
    CorrectionKind, DecisionChoice, DecisionCorrectionDraft, DecisionSupersessionDraft,
    DeterministicIdGenerator, ErrorKind, ExplicitQuestionResponse, FixedClock, MergeResolution,
    MergeResolutionMode, NonUserQuestionOutcome, OperationId, Principal, PrincipalKind,
    QuestionAlternative, QuestionDependency, QuestionDispositionDraft, QuestionDraft,
    QuestionReference, QuestionResponseDraft, QuestionTerminalOutcome, SourceDraft, SourcePayload,
    SourceRelationKind, StatementProvenanceRole, Store, TimestampMicros, UserAcceptanceFact,
    UserAcceptanceState, UserReviewFact, UserReviewState, UserTurnSource, VerificationFact,
    VerificationState, WorkState,
};

fn operation(value: u8) -> OperationId {
    OperationId::from_bytes([value; 16])
}

fn store(path: &Path, ids: &[u8]) -> Result<Store, volicord_context::Error> {
    Store::open_with(
        path,
        DeterministicIdGenerator::new(ids.iter().map(|value| [*value; 16])),
        FixedClock::new(TimestampMicros::from_unix_micros(1_790_000_000_000_000)),
    )
}

fn clone_from(
    path: &Path,
    bundle: &Path,
    operation_id: u8,
) -> Result<Store, Box<dyn std::error::Error>> {
    let mut value = store(path, &[])?;
    value.import_bundle(operation(operation_id), bundle)?;
    Ok(value)
}

fn principal(kind: PrincipalKind, identity: &str) -> Principal {
    Principal {
        kind,
        identity: identity.to_owned(),
    }
}

fn user_turn(turn: &str) -> SourceDraft {
    SourceDraft {
        expected_project_revision: 1,
        payload: SourcePayload::CurrentHostUserTurn {
            host: "codex".to_owned(),
            session: "forgetting-matrix".to_owned(),
            turn: turn.to_owned(),
        },
        actor: principal(PrincipalKind::User, "project-owner"),
        observer: None,
        availability: Availability::Available,
    }
}

struct Fixture {
    project: volicord_context::Project,
    source: volicord_context::Source,
    authorization: volicord_context::Source,
    other_authorization: volicord_context::Source,
    question: volicord_context::Question,
    dependent_question: volicord_context::Question,
    decision: volicord_context::Decision,
    context_item: volicord_context::ContextItem,
    checkpoint: volicord_context::Checkpoint,
}

fn create_fixture(value: &mut Store) -> Result<Fixture, Box<dyn std::error::Error>> {
    let project = value
        .create_project(operation(1), "Forgetting matrix")?
        .value;
    let source = value
        .record_source(
            operation(2),
            project.id,
            SourceDraft {
                expected_project_revision: 1,
                payload: SourcePayload::File {
                    locator: "SOURCE-SECRET-7f91/private.rs".to_owned(),
                    snapshot: "SOURCE-SNAPSHOT-SECRET-7f91".to_owned(),
                },
                actor: principal(PrincipalKind::Repository, "fixture-repository"),
                observer: Some(principal(PrincipalKind::Agent, "codex")),
                availability: Availability::Available,
            },
        )?
        .value;
    let authorization = value
        .record_source(operation(3), project.id, user_turn("forget-authority"))?
        .value;
    let other_authorization = value
        .record_source(operation(4), project.id, user_turn("other-authority"))?
        .value;
    let question = value
        .create_question(
            operation(5),
            project.id,
            QuestionDraft {
                expected_project_revision: 1,
                prompt_basis: "QUESTION-SECRET-91ac choose storage?".to_owned(),
                source_basis: vec![source.id],
                dependencies: vec![],
                alternatives: vec![QuestionAlternative {
                    key: "local".to_owned(),
                    label: "Local".to_owned(),
                    consequence: "QUESTION-ALTERNATIVE-SECRET-91ac".to_owned(),
                }],
                recommendation: AgentRecommendation {
                    alternative_key: Some("local".to_owned()),
                    rationale: "QUESTION-RECOMMENDATION-SECRET-91ac".to_owned(),
                    source_basis: vec![source.id],
                },
                trade_offs: vec!["QUESTION-TRADEOFF-SECRET-91ac".to_owned()],
                uncertainty: vec!["QUESTION-UNCERTAINTY-SECRET-91ac".to_owned()],
                material_scope: vec!["storage".to_owned()],
                materiality: volicord_context::QuestionMateriality::Material,
                presentation_order: 0,
                why_it_matters_now: "storage selection is required".to_owned(),
                established_facts: vec![],
                assumptions: vec![],
                known_limits: vec![],
                what_the_answer_unlocks: vec!["storage work".to_owned()],
                allowed_non_choice_dispositions: volicord_context::NonUserQuestionOutcome::ALL
                    .to_vec(),
                research_state: volicord_context::QuestionResearchState::ReadyToAsk,
            },
        )?
        .value;
    let dependent_question = value
        .create_question(
            operation(6),
            project.id,
            QuestionDraft {
                expected_project_revision: 1,
                prompt_basis: "Retained dependent question".to_owned(),
                source_basis: vec![authorization.id],
                dependencies: vec![QuestionDependency {
                    question_id: question.id,
                    required_revision: 1,
                    required_outcome: QuestionTerminalOutcome::Answered,
                    required_source_basis: vec![],
                    blocked_outcomes: vec![],
                    superseding_outcomes: vec![],
                    assessment_source_basis: vec![authorization.id],
                }],
                alternatives: vec![QuestionAlternative {
                    key: "continue".to_owned(),
                    label: "Continue".to_owned(),
                    consequence: "Continue after dependency".to_owned(),
                }],
                recommendation: AgentRecommendation {
                    alternative_key: None,
                    rationale: "Wait for the prerequisite".to_owned(),
                    source_basis: vec![authorization.id],
                },
                trade_offs: vec![],
                uncertainty: vec![],
                material_scope: vec!["dependent".to_owned()],
                materiality: volicord_context::QuestionMateriality::Material,
                presentation_order: 1,
                why_it_matters_now: "the dependency gates continued work".to_owned(),
                established_facts: vec![],
                assumptions: vec![],
                known_limits: vec![],
                what_the_answer_unlocks: vec!["dependent work".to_owned()],
                allowed_non_choice_dispositions: volicord_context::NonUserQuestionOutcome::ALL
                    .to_vec(),
                research_state: volicord_context::QuestionResearchState::ReadyToAsk,
            },
        )?
        .value;
    let decision = value
        .record_question_response(
            operation(7),
            project.id,
            QuestionResponseDraft {
                expected_project_revision: 1,
                question_id: question.id,
                question_revision: 1,
                user_turn_source: UserTurnSource::Existing(authorization.id),
                displayed_alternative_keys: vec!["local".to_owned()],
                displayed_recommendation_key: Some("local".to_owned()),
                response: ExplicitQuestionResponse::Choice {
                    alternative_key: "local".to_owned(),
                    user_rationale: Some("DECISION-SECRET-2bd4 rationale".to_owned()),
                },
                applicability: ApplicabilityScope::default(),
                assumptions: vec!["DECISION-ASSUMPTION-SECRET-2bd4".to_owned()],
                revisit_triggers: vec!["DECISION-TRIGGER-SECRET-2bd4".to_owned()],
            },
        )?
        .value
        .decision
        .ok_or("fixture Decision missing")?;
    let context_item = value
        .record_context_item(
            operation(8),
            project.id,
            ContextItemDraft {
                expected_project_revision: 1,
                role: ContextItemRole::Fact,
                statement: "CONTEXT-SECRET-53c8 statement".to_owned(),
                provenance_role: StatementProvenanceRole::Observed,
                author: principal(PrincipalKind::Agent, "codex"),
                source_basis: vec![source.id],
                applicability: ApplicabilityScope::default(),
            },
        )?
        .value;
    let checkpoint = value
        .record_checkpoint(
            operation(9),
            project.id,
            CheckpointDraft {
                expected_project_revision: 1,
                kind: CheckpointKind::Handoff,
                goal: "CHECKPOINT-SECRET-e5a2 goal".to_owned(),
                work_state: WorkState::Paused,
                state_change: Some("CHECKPOINT-STATE-SECRET-e5a2".to_owned()),
                source_basis: vec![source.id],
                changed_source_basis: vec![source.id],
                changed_paths: vec!["src/private.rs".to_owned()],
                applied_decisions: vec![decision.id],
                verification: vec![VerificationFact {
                    state: VerificationState::NotRun,
                    source_id: None,
                    outcome: None,
                }],
                user_review: UserReviewFact {
                    state: UserReviewState::Reviewed,
                    source_id: Some(authorization.id),
                },
                user_acceptance: UserAcceptanceFact {
                    state: UserAcceptanceState::NotRequested,
                    source_id: None,
                },
                known_limits: vec!["CHECKPOINT-LIMIT-SECRET-e5a2".to_owned()],
                non_goals: vec!["CHECKPOINT-NONGOAL-SECRET-e5a2".to_owned()],
                open_questions: vec![QuestionReference {
                    question_id: dependent_question.id,
                    revision: 1,
                }],
                next_step: "CHECKPOINT-NEXT-SECRET-e5a2".to_owned(),
                handoff_to: Some("next-agent".to_owned()),
            },
        )?
        .value;
    value.relate_sources(
        operation(10),
        project.id,
        1,
        source.id,
        SourceRelationKind::SupportedBy,
        authorization.id,
    )?;
    value.record_contradiction(
        operation(11),
        project.id,
        CanonicalRecordId::Decision(decision.id),
        CanonicalRecordId::ContextItem(context_item.id),
    )?;
    Ok(Fixture {
        project,
        source,
        authorization,
        other_authorization,
        question,
        dependent_question,
        decision,
        context_item,
        checkpoint,
    })
}

fn assert_kind(error: volicord_context::Error, kind: ErrorKind) {
    assert_eq!(error.kind(), kind, "unexpected error: {error}");
}

#[test]
fn every_content_bearing_kind_forgets_to_one_minimal_portable_tombstone(
) -> Result<(), Box<dyn std::error::Error>> {
    const SECRETS: &[&str] = &[
        "SOURCE-SECRET-7f91",
        "SOURCE-SNAPSHOT-SECRET-7f91",
        "QUESTION-SECRET-91ac",
        "QUESTION-ALTERNATIVE-SECRET-91ac",
        "QUESTION-RECOMMENDATION-SECRET-91ac",
        "QUESTION-TRADEOFF-SECRET-91ac",
        "QUESTION-UNCERTAINTY-SECRET-91ac",
        "DECISION-SECRET-2bd4",
        "DECISION-ASSUMPTION-SECRET-2bd4",
        "DECISION-TRIGGER-SECRET-2bd4",
        "CONTEXT-SECRET-53c8",
        "CHECKPOINT-SECRET-e5a2",
        "CHECKPOINT-STATE-SECRET-e5a2",
        "CHECKPOINT-LIMIT-SECRET-e5a2",
        "CHECKPOINT-NONGOAL-SECRET-e5a2",
        "CHECKPOINT-NEXT-SECRET-e5a2",
    ];
    let root = tempdir()?;
    let database = root.path().join("context.sqlite3");
    let managed_bundle = root.path().join("managed.json");
    let mut value = store(&database, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11])?;
    let fixture = create_fixture(&mut value)?;
    value.export_bundle(fixture.project.id, &managed_bundle)?;
    let other_project = value.create_project(operation(12), "Other Project")?.value;

    assert_kind(
        value
            .forget_source(
                operation(20),
                other_project.id,
                fixture.source.id,
                fixture.authorization.id,
            )
            .err()
            .ok_or("cross-Project Source forgetting succeeded")?,
        ErrorKind::WrongProject,
    );
    assert_kind(
        value
            .forget_question(
                operation(21),
                other_project.id,
                fixture.question.id,
                fixture.authorization.id,
            )
            .err()
            .ok_or("cross-Project Question forgetting succeeded")?,
        ErrorKind::WrongProject,
    );
    assert_kind(
        value
            .forget_decision(
                operation(22),
                other_project.id,
                fixture.decision.id,
                fixture.authorization.id,
            )
            .err()
            .ok_or("cross-Project Decision forgetting succeeded")?,
        ErrorKind::WrongProject,
    );
    assert_kind(
        value
            .forget_context_item(
                operation(23),
                other_project.id,
                fixture.context_item.id,
                fixture.authorization.id,
            )
            .err()
            .ok_or("cross-Project Context Item forgetting succeeded")?,
        ErrorKind::WrongProject,
    );
    assert_kind(
        value
            .forget_checkpoint(
                operation(24),
                other_project.id,
                fixture.checkpoint.id,
                fixture.authorization.id,
            )
            .err()
            .ok_or("cross-Project Checkpoint forgetting succeeded")?,
        ErrorKind::WrongProject,
    );
    assert_kind(
        value
            .forget_source(
                operation(25),
                fixture.project.id,
                fixture.source.id,
                fixture.source.id,
            )
            .err()
            .ok_or("repository Source authorized forgetting")?,
        ErrorKind::InvalidInput,
    );

    let source_result = value.forget_source(
        operation(30),
        fixture.project.id,
        fixture.source.id,
        fixture.authorization.id,
    )?;
    assert_eq!(
        source_result.value.tombstone.record,
        CanonicalRecordId::Source(fixture.source.id)
    );
    assert!(
        value
            .forget_source(
                operation(30),
                fixture.project.id,
                fixture.source.id,
                fixture.authorization.id,
            )?
            .replayed
    );
    assert_kind(
        value
            .forget_source(
                operation(30),
                fixture.project.id,
                fixture.source.id,
                fixture.other_authorization.id,
            )
            .err()
            .ok_or("changed forgetting replay input succeeded")?,
        ErrorKind::DomainConflict,
    );
    assert!(value
        .get_source_relation(
            fixture.project.id,
            fixture.source.id,
            SourceRelationKind::SupportedBy,
            fixture.authorization.id,
        )
        .is_err());

    value.forget_question(
        operation(31),
        fixture.project.id,
        fixture.question.id,
        fixture.authorization.id,
    )?;
    assert!(value
        .get_question(fixture.project.id, fixture.question.id)
        .is_err());
    assert!(value
        .get_question(fixture.project.id, fixture.dependent_question.id)?
        .dependencies
        .is_empty());
    assert_eq!(
        value
            .get_decision(fixture.project.id, fixture.decision.id)?
            .question_id,
        fixture.question.id
    );

    value.forget_decision(
        operation(32),
        fixture.project.id,
        fixture.decision.id,
        fixture.authorization.id,
    )?;
    value.forget_context_item(
        operation(33),
        fixture.project.id,
        fixture.context_item.id,
        fixture.authorization.id,
    )?;
    value.forget_checkpoint(
        operation(34),
        fixture.project.id,
        fixture.checkpoint.id,
        fixture.authorization.id,
    )?;
    assert!(
        value
            .forget_checkpoint(
                operation(34),
                fixture.project.id,
                fixture.checkpoint.id,
                fixture.authorization.id,
            )?
            .replayed
    );
    assert_kind(
        value
            .correct_decision(
                operation(35),
                fixture.project.id,
                fixture.decision.id,
                DecisionCorrectionDraft {
                    expected_revision: 1,
                    corrected_user_rationale: Some("safe correction".to_owned()),
                    kind: CorrectionKind::Formatting,
                    user_authorization_source_id: fixture.authorization.id,
                },
            )
            .err()
            .ok_or("correction reconstructed a forgotten Decision")?,
        ErrorKind::NotFound,
    );
    assert_kind(
        value
            .supersede_decision(
                operation(36),
                fixture.project.id,
                DecisionSupersessionDraft {
                    expected_project_revision: 1,
                    previous_decision_id: fixture.decision.id,
                    user_turn_source: UserTurnSource::Existing(fixture.authorization.id),
                    choice: DecisionChoice::Alternative {
                        alternative_key: "local".to_owned(),
                    },
                    user_rationale: None,
                    applicability: ApplicabilityScope::default(),
                    assumptions: vec![],
                    revisit_triggers: vec![],
                },
            )
            .err()
            .ok_or("supersession reconstructed a forgotten Decision")?,
        ErrorKind::NotFound,
    );
    assert_kind(
        value
            .correct_context_item(
                operation(37),
                fixture.project.id,
                fixture.context_item.id,
                ContextItemCorrectionDraft {
                    expected_revision: 1,
                    corrected_statement: "safe correction".to_owned(),
                    kind: CorrectionKind::Formatting,
                    user_authorization_source_id: fixture.authorization.id,
                },
            )
            .err()
            .ok_or("correction reconstructed a forgotten Context Item")?,
        ErrorKind::NotFound,
    );

    let relation = value.get_canonical_relation(
        fixture.project.id,
        CanonicalRecordId::Decision(fixture.decision.id),
        CanonicalRelationKind::Contradicts,
        CanonicalRecordId::ContextItem(fixture.context_item.id),
    )?;
    assert_eq!(
        relation.from,
        CanonicalRecordId::Decision(fixture.decision.id)
    );
    let basis = value.read_canonical_basis(
        fixture.project.id,
        CanonicalReadOptions {
            include_checkpoint_history: true,
        },
    )?;
    assert!(basis
        .sources
        .iter()
        .all(|entry| entry.source.id != fixture.source.id));
    assert!(basis
        .active_questions
        .iter()
        .all(|entry| entry.id != fixture.question.id));
    assert!(basis
        .active_decisions
        .iter()
        .all(|entry| entry.decision.id != fixture.decision.id));
    assert!(basis
        .context_items
        .iter()
        .all(|entry| entry.id != fixture.context_item.id));
    assert!(basis
        .checkpoint_history
        .iter()
        .all(|entry| entry.id != fixture.checkpoint.id));
    let expected = [
        (CanonicalRecordKind::Source, fixture.source.id.to_string()),
        (
            CanonicalRecordKind::Question,
            fixture.question.id.to_string(),
        ),
        (
            CanonicalRecordKind::Decision,
            fixture.decision.id.to_string(),
        ),
        (
            CanonicalRecordKind::ContextItem,
            fixture.context_item.id.to_string(),
        ),
        (
            CanonicalRecordKind::Checkpoint,
            fixture.checkpoint.id.to_string(),
        ),
    ];
    for (kind, identity) in expected {
        assert!(basis
            .forgotten
            .iter()
            .any(|entry| entry.record_kind == kind && entry.record_identity == identity));
    }
    assert!(basis
        .forgotten
        .iter()
        .all(|entry| entry.record_kind != CanonicalRecordKind::Project));
    drop(value);

    let reopened = store(&database, &[])?;
    let reopened_basis =
        reopened.read_canonical_basis(fixture.project.id, CanonicalReadOptions::default())?;
    assert_eq!(reopened_basis.forgotten.len(), 5);
    drop(reopened);

    let managed_bytes = fs::read(&managed_bundle)?;
    for secret in SECRETS {
        assert!(!managed_bytes
            .windows(secret.len())
            .any(|window| window == secret.as_bytes()));
    }
    for entry in fs::read_dir(root.path())? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let bytes = fs::read(entry.path())?;
            for secret in SECRETS {
                assert!(
                    !bytes
                        .windows(secret.len())
                        .any(|window| window == secret.as_bytes()),
                    "forgotten bytes remain in {}",
                    entry.path().display()
                );
            }
        }
    }
    let connection = Connection::open(&database)?;
    let tombstone_columns: Vec<String> = connection
        .prepare("PRAGMA table_info(tombstones)")?
        .query_map([], |row| row.get(1))?
        .collect::<Result<_, _>>()?;
    assert_eq!(
        tombstone_columns,
        vec!["project_id", "record_kind", "record_id", "forgotten_at"]
    );
    drop(connection);

    let imported_database = root.path().join("imported.sqlite3");
    let imported_bundle = root.path().join("imported.json");
    let imported_repeat = root.path().join("imported-repeat.json");
    let mut imported = store(&imported_database, &[])?;
    imported.import_bundle(operation(40), &managed_bundle)?;
    let imported_basis =
        imported.read_canonical_basis(fixture.project.id, CanonicalReadOptions::default())?;
    assert_eq!(imported_basis.forgotten, reopened_basis.forgotten);
    imported.export_bundle(fixture.project.id, &imported_bundle)?;
    imported.export_bundle(fixture.project.id, &imported_repeat)?;
    assert_eq!(fs::read(&imported_bundle)?, fs::read(&imported_repeat)?);
    Ok(())
}

#[test]
fn source_question_and_checkpoint_forgetting_propagate_through_merge(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut origin = store(
        &root.path().join("merge-origin.sqlite3"),
        &[1, 2, 3, 4, 5, 6, 7, 8, 9],
    )?;
    let fixture = create_fixture(&mut origin)?;
    let base_bundle = root.path().join("merge-base.json");
    origin.export_bundle(fixture.project.id, &base_bundle)?;

    let mut source_local =
        clone_from(&root.path().join("source-local.sqlite3"), &base_bundle, 100)?;
    source_local.forget_source(
        operation(101),
        fixture.project.id,
        fixture.source.id,
        fixture.authorization.id,
    )?;
    let mut source_incoming = clone_from(
        &root.path().join("source-incoming.sqlite3"),
        &base_bundle,
        102,
    )?;
    source_incoming.relate_sources(
        operation(103),
        fixture.project.id,
        1,
        fixture.source.id,
        SourceRelationKind::DerivedFrom,
        fixture.other_authorization.id,
    )?;
    let source_incoming_bundle = root.path().join("source-incoming.json");
    source_incoming.export_bundle(fixture.project.id, &source_incoming_bundle)?;
    let source_comparison =
        source_local.compare_bundle(Some(&base_bundle), &source_incoming_bundle, None)?;
    assert!(
        source_comparison.conflicts.iter().any(|conflict| {
            conflict.class == BundleConflictClass::DeleteModifyConflict
                && conflict
                    .affected_identities
                    .iter()
                    .any(|identity| identity.ends_with(&fixture.source.id.to_string()))
        }),
        "source conflicts: {:?}",
        source_comparison.conflicts
    );
    source_local.merge_bundle(
        operation(104),
        Some(&base_bundle),
        &source_incoming_bundle,
        None,
        Some(MergeResolution {
            conflict_set_identity: source_comparison.conflict_set_identity,
            conflict_revision: 1,
            user_turn_source_id: fixture.authorization.id,
            mode: MergeResolutionMode::ChooseLocal,
        }),
    )?;
    assert!(source_local
        .get_source(fixture.project.id, fixture.source.id)
        .is_err());
    assert_eq!(
        source_local
            .get_tombstone(
                fixture.project.id,
                CanonicalRecordId::Source(fixture.source.id)
            )?
            .record,
        CanonicalRecordId::Source(fixture.source.id)
    );

    let mut question_local = clone_from(
        &root.path().join("question-local.sqlite3"),
        &base_bundle,
        110,
    )?;
    question_local.forget_question(
        operation(111),
        fixture.project.id,
        fixture.dependent_question.id,
        fixture.authorization.id,
    )?;
    let mut question_incoming = clone_from(
        &root.path().join("question-incoming.sqlite3"),
        &base_bundle,
        112,
    )?;
    question_incoming.dispose_question(
        operation(113),
        fixture.project.id,
        QuestionDispositionDraft {
            expected_project_revision: 1,
            question_id: fixture.dependent_question.id,
            question_revision: 1,
            outcome: NonUserQuestionOutcome::Deferred,
            source_basis: vec![fixture.authorization.id],
            reason: "defer the dependent branch".to_owned(),
            replacement_question_id: None,
            revisit_basis: vec!["after dependency review".to_owned()],
            actor: principal(PrincipalKind::Agent, "inquiry"),
        },
    )?;
    let question_incoming_bundle = root.path().join("question-incoming.json");
    question_incoming.export_bundle(fixture.project.id, &question_incoming_bundle)?;
    let question_comparison =
        question_local.compare_bundle(Some(&base_bundle), &question_incoming_bundle, None)?;
    assert!(question_comparison.conflicts.iter().any(|conflict| {
        conflict.class == BundleConflictClass::DeleteModifyConflict
            && conflict
                .affected_identities
                .iter()
                .any(|identity| identity.ends_with(&fixture.dependent_question.id.to_string()))
    }));
    question_local.merge_bundle(
        operation(114),
        Some(&base_bundle),
        &question_incoming_bundle,
        None,
        Some(MergeResolution {
            conflict_set_identity: question_comparison.conflict_set_identity,
            conflict_revision: 1,
            user_turn_source_id: fixture.authorization.id,
            mode: MergeResolutionMode::ChooseLocal,
        }),
    )?;
    assert!(question_local
        .get_question(fixture.project.id, fixture.dependent_question.id)
        .is_err());

    let mut checkpoint_local = clone_from(
        &root.path().join("checkpoint-local.sqlite3"),
        &base_bundle,
        120,
    )?;
    checkpoint_local.forget_checkpoint(
        operation(121),
        fixture.project.id,
        fixture.checkpoint.id,
        fixture.authorization.id,
    )?;
    let mut checkpoint_incoming = clone_from(
        &root.path().join("checkpoint-incoming.sqlite3"),
        &base_bundle,
        122,
    )?;
    checkpoint_incoming.correct_context_item(
        operation(123),
        fixture.project.id,
        fixture.context_item.id,
        ContextItemCorrectionDraft {
            expected_revision: 1,
            corrected_statement: "CONTEXT-SECRET-53c8  statement".to_owned(),
            kind: CorrectionKind::Formatting,
            user_authorization_source_id: fixture.authorization.id,
        },
    )?;
    let checkpoint_incoming_bundle = root.path().join("checkpoint-incoming.json");
    checkpoint_incoming.export_bundle(fixture.project.id, &checkpoint_incoming_bundle)?;
    let checkpoint_comparison =
        checkpoint_local.compare_bundle(Some(&base_bundle), &checkpoint_incoming_bundle, None)?;
    assert!(!checkpoint_comparison.requires_user_resolution());
    let merged = checkpoint_local.merge_bundle(
        operation(124),
        Some(&base_bundle),
        &checkpoint_incoming_bundle,
        None,
        None,
    )?;
    assert_eq!(merged.value.status, BundleMergeStatus::MergedAutomatically);
    assert!(checkpoint_local
        .get_checkpoint(fixture.project.id, fixture.checkpoint.id)
        .is_err());
    assert_eq!(
        checkpoint_local
            .get_context_item(fixture.project.id, fixture.context_item.id)?
            .revision,
        2
    );
    Ok(())
}

#[test]
fn new_forgetting_kinds_roll_back_their_complete_closure_and_retry_safely(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let database = root.path().join("rollback.sqlite3");
    let mut value = store(&database, &[1, 2, 3, 4, 5, 6, 7, 8, 9])?;
    let fixture = create_fixture(&mut value)?;
    drop(value);

    let connection = Connection::open(&database)?;
    connection.execute_batch(
        "CREATE TRIGGER fail_source_forget BEFORE DELETE ON sources
         BEGIN SELECT RAISE(ABORT, 'source forget fault'); END;",
    )?;
    drop(connection);
    let mut value = store(&database, &[])?;
    assert_kind(
        value
            .forget_source(
                operation(50),
                fixture.project.id,
                fixture.source.id,
                fixture.authorization.id,
            )
            .err()
            .ok_or("faulted Source forgetting succeeded")?,
        ErrorKind::TransactionFailure,
    );
    assert_eq!(
        value.get_source(fixture.project.id, fixture.source.id)?.id,
        fixture.source.id
    );
    assert!(value
        .get_tombstone(
            fixture.project.id,
            CanonicalRecordId::Source(fixture.source.id)
        )
        .is_err());
    drop(value);

    let connection = Connection::open(&database)?;
    connection.execute_batch(
        "DROP TRIGGER fail_source_forget;
         CREATE TRIGGER fail_question_forget BEFORE DELETE ON questions
         BEGIN SELECT RAISE(ABORT, 'question forget fault'); END;",
    )?;
    drop(connection);
    let mut value = store(&database, &[])?;
    assert_kind(
        value
            .forget_question(
                operation(51),
                fixture.project.id,
                fixture.question.id,
                fixture.authorization.id,
            )
            .err()
            .ok_or("faulted Question forgetting succeeded")?,
        ErrorKind::TransactionFailure,
    );
    assert_eq!(
        value
            .get_question(fixture.project.id, fixture.question.id)?
            .id,
        fixture.question.id
    );
    assert!(!value
        .get_question(fixture.project.id, fixture.dependent_question.id)?
        .dependencies
        .is_empty());
    drop(value);

    let connection = Connection::open(&database)?;
    connection.execute_batch(
        "DROP TRIGGER fail_question_forget;
         CREATE TRIGGER fail_checkpoint_forget BEFORE DELETE ON checkpoints
         BEGIN SELECT RAISE(ABORT, 'checkpoint forget fault'); END;",
    )?;
    drop(connection);
    let mut value = store(&database, &[])?;
    assert_kind(
        value
            .forget_checkpoint(
                operation(52),
                fixture.project.id,
                fixture.checkpoint.id,
                fixture.authorization.id,
            )
            .err()
            .ok_or("faulted Checkpoint forgetting succeeded")?,
        ErrorKind::TransactionFailure,
    );
    assert_eq!(
        value
            .get_checkpoint(fixture.project.id, fixture.checkpoint.id)?
            .id,
        fixture.checkpoint.id
    );
    drop(value);

    let connection = Connection::open(&database)?;
    connection.execute_batch("DROP TRIGGER fail_checkpoint_forget;")?;
    drop(connection);
    let mut value = store(&database, &[])?;
    assert!(
        !value
            .forget_checkpoint(
                operation(52),
                fixture.project.id,
                fixture.checkpoint.id,
                fixture.authorization.id,
            )?
            .replayed
    );
    Ok(())
}

#[test]
fn supersession_source_payload_is_purged_by_source_only_forgetting(
) -> Result<(), Box<dyn std::error::Error>> {
    const SECRET: &str = "SUPERSESSION-SOURCE-LEAK-6f3d";
    let root = tempdir()?;
    let database = root.path().join("supersession-source.sqlite3");
    let managed_bundle = root.path().join("supersession-source.json");
    let mut value = store(&database, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11])?;
    let fixture = create_fixture(&mut value)?;
    let supersession_draft = DecisionSupersessionDraft {
        expected_project_revision: 1,
        previous_decision_id: fixture.decision.id,
        user_turn_source: UserTurnSource::Create(user_turn(SECRET)),
        choice: DecisionChoice::Alternative {
            alternative_key: "local".to_owned(),
        },
        user_rationale: Some("independent user rationale".to_owned()),
        applicability: ApplicabilityScope::default(),
        assumptions: vec![],
        revisit_triggers: vec![],
    };
    let supersession = value
        .supersede_decision(
            operation(90),
            fixture.project.id,
            supersession_draft.clone(),
        )?
        .value;
    let missing_dependencies: i64 = Connection::open(&database)?.query_row(
        "SELECT COUNT(*) FROM operations AS operation
         WHERE replay_state = 'available' AND NOT EXISTS (
             SELECT 1 FROM operation_dependencies AS dependency
             WHERE dependency.operation_id = operation.operation_id
         )",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(missing_dependencies, 0);
    value.export_bundle(fixture.project.id, &managed_bundle)?;
    value.forget_source(
        operation(91),
        fixture.project.id,
        supersession.user_turn_source_id,
        fixture.authorization.id,
    )?;
    assert_kind(
        value
            .supersede_decision(operation(90), fixture.project.id, supersession_draft)
            .err()
            .ok_or("forgotten Source replay reported success")?,
        ErrorKind::NotFound,
    );
    let operation_state: (Vec<u8>, String) = Connection::open(&database)?.query_row(
        "SELECT input_basis, replay_state FROM operations WHERE operation_id = ?1",
        [operation(90).as_bytes().as_slice()],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    assert!(operation_state.0.is_empty());
    assert_eq!(operation_state.1, "forgotten_dependency");
    drop(value);

    for entry in fs::read_dir(root.path())? {
        let path = entry?.path();
        if path.is_file() {
            let bytes = fs::read(&path)?;
            assert!(
                !bytes
                    .windows(SECRET.len())
                    .any(|window| window == SECRET.as_bytes()),
                "forgotten supersession Source remains in {}",
                path.display()
            );
        }
    }
    Ok(())
}

#[test]
fn question_only_forgetting_purges_owned_decision_presentation(
) -> Result<(), Box<dyn std::error::Error>> {
    const ALTERNATIVE_SECRET: &str = "QUESTION-ALTERNATIVE-SECRET-91ac";
    const RECOMMENDATION_SECRET: &str = "QUESTION-RECOMMENDATION-SECRET-91ac";
    let root = tempdir()?;
    let database = root.path().join("question-copy.sqlite3");
    let managed_bundle = root.path().join("question-copy.json");
    let mut value = store(&database, &[1, 2, 3, 4, 5, 6, 7, 8, 9])?;
    let fixture = create_fixture(&mut value)?;
    value.export_bundle(fixture.project.id, &managed_bundle)?;
    value.forget_question(
        operation(92),
        fixture.project.id,
        fixture.question.id,
        fixture.authorization.id,
    )?;
    let decision = value.get_decision(fixture.project.id, fixture.decision.id)?;
    assert_eq!(decision.choice, fixture.decision.choice);
    assert_eq!(decision.user_rationale, fixture.decision.user_rationale);
    assert_eq!(decision.applicability, fixture.decision.applicability);
    assert_eq!(
        decision.user_turn_source_id,
        fixture.decision.user_turn_source_id
    );
    assert!(decision.displayed_alternatives.is_empty());
    assert_eq!(decision.displayed_recommendation.alternative_key, None);
    assert!(decision.displayed_recommendation.rationale.is_empty());
    assert!(decision.displayed_recommendation.source_basis.is_empty());
    assert!(value
        .get_decision_lifecycle(fixture.project.id, fixture.decision.id)?
        .review_due
        .is_some());
    let revision_copy_count: i64 = Connection::open(&database)?.query_row(
        "SELECT COUNT(*) FROM decision_revisions
         WHERE decision_id = ?1 AND (
             length(displayed_alternatives) != 8 OR recommendation_key IS NOT NULL OR
             recommendation_rationale != '' OR length(recommendation_sources) != 8
         )",
        [fixture.decision.id.as_bytes().as_slice()],
        |row| row.get(0),
    )?;
    assert_eq!(revision_copy_count, 0);
    drop(value);

    for entry in fs::read_dir(root.path())? {
        let path = entry?.path();
        if path.is_file() {
            let bytes = fs::read(&path)?;
            for secret in [ALTERNATIVE_SECRET, RECOMMENDATION_SECRET] {
                assert!(
                    !bytes
                        .windows(secret.len())
                        .any(|window| window == secret.as_bytes()),
                    "forgotten Question presentation remains in {}",
                    path.display()
                );
            }
        }
    }
    let mut reopened = store(&database, &[])?;
    let reopened_decision = reopened.get_decision(fixture.project.id, fixture.decision.id)?;
    assert!(reopened_decision.displayed_alternatives.is_empty());
    let clean_database = root.path().join("question-copy-import.sqlite3");
    let mut imported = store(&clean_database, &[])?;
    imported.import_bundle(operation(93), &managed_bundle)?;
    let imported_decision = imported.get_decision(fixture.project.id, fixture.decision.id)?;
    assert_eq!(imported_decision, reopened_decision);
    let repeat_bundle = root.path().join("question-copy-repeat.json");
    reopened.export_bundle(fixture.project.id, &repeat_bundle)?;
    assert_eq!(fs::read(&managed_bundle)?, fs::read(&repeat_bundle)?);
    Ok(())
}

#[test]
fn incoming_forgetting_sanitizes_every_supported_local_record_closure(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut origin = store(
        &root.path().join("matrix-origin.sqlite3"),
        &[1, 2, 3, 4, 5, 6, 7, 8, 9],
    )?;
    let fixture = create_fixture(&mut origin)?;
    let base_bundle = root.path().join("matrix-base.json");
    origin.export_bundle(fixture.project.id, &base_bundle)?;
    let scenarios = [
        (
            "source",
            CanonicalRecordId::Source(fixture.source.id),
            "SOURCE-SECRET-7f91",
            Some(131_u8),
        ),
        (
            "question",
            CanonicalRecordId::Question(fixture.dependent_question.id),
            "Retained dependent question",
            Some(132_u8),
        ),
        (
            "decision",
            CanonicalRecordId::Decision(fixture.decision.id),
            "DECISION-SECRET-2bd4",
            Some(133_u8),
        ),
        (
            "context-item",
            CanonicalRecordId::ContextItem(fixture.context_item.id),
            "CONTEXT-SECRET-53c8",
            Some(134_u8),
        ),
        (
            "checkpoint",
            CanonicalRecordId::Checkpoint(fixture.checkpoint.id),
            "CHECKPOINT-SECRET-e5a2",
            None,
        ),
    ];

    for (index, (label, record, secret, local_operation)) in scenarios.into_iter().enumerate() {
        let local_database = root.path().join(format!("{label}-local.sqlite3"));
        let local_bundle = root.path().join(format!("{label}-managed.json"));
        let incoming_database = root.path().join(format!("{label}-incoming.sqlite3"));
        let incoming_bundle = root.path().join(format!("{label}-incoming.json"));
        let mut local = clone_from(&local_database, &base_bundle, 140 + index as u8)?;
        match record {
            CanonicalRecordId::Source(source_id) => {
                local.relate_sources(
                    operation(local_operation.ok_or("Source operation missing")?),
                    fixture.project.id,
                    1,
                    source_id,
                    SourceRelationKind::DerivedFrom,
                    fixture.other_authorization.id,
                )?;
            }
            CanonicalRecordId::Question(question_id) => {
                local.dispose_question(
                    operation(local_operation.ok_or("Question operation missing")?),
                    fixture.project.id,
                    QuestionDispositionDraft {
                        expected_project_revision: 1,
                        question_id,
                        question_revision: 1,
                        outcome: NonUserQuestionOutcome::Deferred,
                        source_basis: vec![fixture.authorization.id],
                        reason: "defer before forgetting".to_owned(),
                        replacement_question_id: None,
                        revisit_basis: vec!["after review".to_owned()],
                        actor: principal(PrincipalKind::Agent, "inquiry"),
                    },
                )?;
            }
            CanonicalRecordId::Decision(decision_id) => {
                local.correct_decision(
                    operation(local_operation.ok_or("Decision operation missing")?),
                    fixture.project.id,
                    decision_id,
                    DecisionCorrectionDraft {
                        expected_revision: 1,
                        corrected_user_rationale: Some(
                            "DECISION-SECRET-2bd4  rationale".to_owned(),
                        ),
                        kind: CorrectionKind::Formatting,
                        user_authorization_source_id: fixture.authorization.id,
                    },
                )?;
            }
            CanonicalRecordId::ContextItem(item_id) => {
                local.correct_context_item(
                    operation(local_operation.ok_or("Context Item operation missing")?),
                    fixture.project.id,
                    item_id,
                    ContextItemCorrectionDraft {
                        expected_revision: 1,
                        corrected_statement: "CONTEXT-SECRET-53c8  statement".to_owned(),
                        kind: CorrectionKind::Formatting,
                        user_authorization_source_id: fixture.authorization.id,
                    },
                )?;
            }
            CanonicalRecordId::Checkpoint(_) => {}
            CanonicalRecordId::Project(_) => return Err("Project is not forgettable".into()),
        }
        local.export_bundle(fixture.project.id, &local_bundle)?;

        let mut incoming = clone_from(&incoming_database, &base_bundle, 150 + index as u8)?;
        match record {
            CanonicalRecordId::Source(id) => {
                incoming.forget_source(
                    operation(160 + index as u8),
                    fixture.project.id,
                    id,
                    fixture.authorization.id,
                )?;
            }
            CanonicalRecordId::Question(id) => {
                incoming.forget_question(
                    operation(160 + index as u8),
                    fixture.project.id,
                    id,
                    fixture.authorization.id,
                )?;
            }
            CanonicalRecordId::Decision(id) => {
                incoming.forget_decision(
                    operation(160 + index as u8),
                    fixture.project.id,
                    id,
                    fixture.authorization.id,
                )?;
            }
            CanonicalRecordId::ContextItem(id) => {
                incoming.forget_context_item(
                    operation(160 + index as u8),
                    fixture.project.id,
                    id,
                    fixture.authorization.id,
                )?;
            }
            CanonicalRecordId::Checkpoint(id) => {
                incoming.forget_checkpoint(
                    operation(160 + index as u8),
                    fixture.project.id,
                    id,
                    fixture.authorization.id,
                )?;
            }
            CanonicalRecordId::Project(_) => return Err("Project is not forgettable".into()),
        }
        incoming.export_bundle(fixture.project.id, &incoming_bundle)?;
        let comparison = local.compare_bundle(Some(&base_bundle), &incoming_bundle, None)?;
        let resolution = comparison
            .requires_user_resolution()
            .then_some(MergeResolution {
                conflict_set_identity: comparison.conflict_set_identity,
                conflict_revision: 1,
                user_turn_source_id: fixture.authorization.id,
                mode: MergeResolutionMode::ChooseIncoming,
            });
        let merge_operation = operation(170 + index as u8);
        local.merge_bundle(
            merge_operation,
            Some(&base_bundle),
            &incoming_bundle,
            None,
            resolution,
        )?;
        assert_eq!(
            local.get_tombstone(fixture.project.id, record)?.record,
            record
        );
        let connection = Connection::open(&local_database)?;
        let sanitation_state: String = connection.query_row(
            "SELECT sanitation_state FROM merge_sanitation WHERE operation_id = ?1",
            [merge_operation.as_bytes().as_slice()],
            |row| row.get(0),
        )?;
        assert_eq!(sanitation_state, "complete");
        let merge_basis: Vec<u8> = connection.query_row(
            "SELECT input_basis FROM operations WHERE operation_id = ?1",
            [merge_operation.as_bytes().as_slice()],
            |row| row.get(0),
        )?;
        assert!(merge_basis.is_empty());
        if let Some(value) = local_operation {
            let local_basis: Vec<u8> = connection.query_row(
                "SELECT input_basis FROM operations WHERE operation_id = ?1",
                [operation(value).as_bytes().as_slice()],
                |row| row.get(0),
            )?;
            assert!(local_basis.is_empty());
        }
        drop(connection);
        drop(local);
        for path in [&local_database, &local_bundle] {
            let bytes = fs::read(path)?;
            assert!(
                !bytes
                    .windows(secret.len())
                    .any(|window| window == secret.as_bytes()),
                "{label} merge-selected forgetting left content in {}",
                path.display()
            );
        }
    }
    Ok(())
}
