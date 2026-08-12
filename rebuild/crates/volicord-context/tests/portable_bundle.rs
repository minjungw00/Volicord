use rusqlite::{params, Connection};
use std::fs;
use std::path::Path;
use tempfile::tempdir;
use volicord_context::{
    AgentRecommendation, ApplicabilityScope, Availability, BundleImportStatus, CanonicalRecordId,
    CanonicalRelationKind, CheckpointDraft, CheckpointKind, ContextItemCorrectionDraft,
    ContextItemDraft, ContextItemRole, CorrectionKind, DecisionChoice, DecisionSupersessionDraft,
    DeterministicIdGenerator, ErrorKind, ExplicitQuestionResponse, FixedClock, OperationId,
    Principal, PrincipalKind, Project, QuestionAlternative, QuestionDraft, QuestionResponseDraft,
    Source, SourceDraft, SourcePayload, StatementProvenanceRole, Store, TimestampMicros,
    UserAcceptanceFact, UserAcceptanceState, UserReviewFact, UserReviewState, UserTurnSource,
    VerificationFact, VerificationState, WorkState, BUNDLE_FORMAT_VERSION, BUNDLE_KIND,
};

fn operation(value: u8) -> OperationId {
    OperationId::from_bytes([value; 16])
}

fn test_store(path: &Path, ids: &[u8]) -> Result<Store, volicord_context::Error> {
    Store::open_with(
        path,
        DeterministicIdGenerator::new(ids.iter().map(|value| [*value; 16])),
        FixedClock::new(TimestampMicros::from_unix_micros(1_760_000_000_000_000)),
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
        actor: principal(actor, "portable-author"),
        observer: Some(principal(PrincipalKind::Agent, "codex")),
        availability: Availability::Available,
    }
}

fn user_turn(turn: &str) -> SourceDraft {
    source_draft(
        SourcePayload::CurrentHostUserTurn {
            host: "codex".to_owned(),
            session: "portable-session".to_owned(),
            turn: turn.to_owned(),
        },
        PrincipalKind::User,
    )
}

struct Fixture {
    project: Project,
    repository: Source,
    item: volicord_context::ContextItem,
    decision: volicord_context::Decision,
    checkpoint: volicord_context::Checkpoint,
    authorization: Source,
}

fn populate(
    store: &mut Store,
    clone_path: &Path,
    statement: &str,
) -> Result<Fixture, Box<dyn std::error::Error>> {
    let project = store
        .create_project(operation(1), "Portable Context")?
        .value;
    store.bind_clone(
        operation(2),
        project.id,
        None,
        clone_path,
        Availability::Available,
    )?;
    let repository = store
        .record_source(
            operation(3),
            project.id,
            source_draft(
                SourcePayload::File {
                    locator: "src/lib.rs".to_owned(),
                    snapshot: "commit-a".to_owned(),
                },
                PrincipalKind::Repository,
            ),
        )?
        .value;
    let authorization = store
        .record_source(operation(4), project.id, user_turn("authorization"))?
        .value;
    let item = store
        .record_context_item(
            operation(5),
            project.id,
            ContextItemDraft {
                expected_project_revision: 1,
                role: ContextItemRole::Fact,
                statement: statement.to_owned(),
                provenance_role: StatementProvenanceRole::Observed,
                author: principal(PrincipalKind::Agent, "observer"),
                source_basis: vec![repository.id],
                applicability: ApplicabilityScope {
                    paths: vec!["rebuild/".to_owned()],
                    components: vec!["context".to_owned()],
                    work_contexts: vec![],
                },
            },
        )?
        .value;
    let question = store
        .create_question(
            operation(6),
            project.id,
            QuestionDraft {
                expected_project_revision: 1,
                prompt_basis: "Choose portability".to_owned(),
                source_basis: vec![repository.id],
                dependencies: vec![],
                alternatives: vec![QuestionAlternative {
                    key: "json".to_owned(),
                    label: "JSON".to_owned(),
                    consequence: "Portable text".to_owned(),
                }],
                recommendation: AgentRecommendation {
                    alternative_key: Some("json".to_owned()),
                    rationale: "Deterministic".to_owned(),
                    source_basis: vec![repository.id],
                },
                trade_offs: vec!["size".to_owned()],
                uncertainty: vec![],
                material_scope: vec!["bundle".to_owned()],
            },
        )?
        .value;
    let response = store
        .record_question_response(
            operation(7),
            project.id,
            QuestionResponseDraft {
                expected_project_revision: 1,
                question_id: question.id,
                question_revision: question.revision,
                user_turn_source: UserTurnSource::Create(user_turn("decision")),
                displayed_alternative_keys: vec!["json".to_owned()],
                displayed_recommendation_key: Some("json".to_owned()),
                response: ExplicitQuestionResponse::Choice {
                    alternative_key: "json".to_owned(),
                    user_rationale: Some("Readable and deterministic".to_owned()),
                },
                applicability: ApplicabilityScope::default(),
                assumptions: vec![],
                revisit_triggers: vec![],
            },
        )?
        .value;
    let decision = response.decision.ok_or("missing Decision")?;
    let checkpoint = store
        .record_checkpoint(
            operation(8),
            project.id,
            CheckpointDraft {
                expected_project_revision: 1,
                kind: CheckpointKind::Completion,
                goal: "Portable context implemented".to_owned(),
                work_state: WorkState::Completed,
                state_change: Some("Bundle ready".to_owned()),
                source_basis: vec![repository.id],
                changed_source_basis: vec![repository.id],
                changed_paths: vec!["rebuild/crates/volicord-context".to_owned()],
                applied_decisions: vec![decision.id],
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
                known_limits: vec!["No divergent merge".to_owned()],
                non_goals: vec!["Remote sync".to_owned()],
                open_questions: vec![],
                next_step: "Import elsewhere".to_owned(),
                handoff_to: None,
            },
        )?
        .value;
    Ok(Fixture {
        project,
        repository,
        item,
        decision,
        checkpoint,
        authorization,
    })
}

#[test]
fn repeated_export_is_identical_and_excludes_local_and_noncanonical_classes(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let clone = root.path().join("private-absolute-clone");
    fs::create_dir(&clone)?;
    let mut store = test_store(
        &root.path().join("source.sqlite3"),
        &[1, 2, 3, 4, 5, 6, 7, 8, 9],
    )?;
    let fixture = populate(&mut store, &clone, "portable fact")?;
    assert_eq!(
        store
            .record_source(
                operation(99),
                fixture.project.id,
                source_draft(
                    SourcePayload::File {
                        locator: clone.join("secret.rs").to_string_lossy().into_owned(),
                        snapshot: "commit-a".to_owned(),
                    },
                    PrincipalKind::Repository,
                ),
            )
            .err()
            .ok_or("expected local absolute locator rejection")?
            .kind(),
        ErrorKind::InvalidInput
    );
    let first = root.path().join("first.json");
    let second = root.path().join("second.json");
    let first_result = store.export_bundle(fixture.project.id, &first)?;
    let second_result = store.export_bundle(fixture.project.id, &second)?;
    assert_eq!(fs::read(&first)?, fs::read(&second)?);
    assert_eq!(first_result.checksum, second_result.checksum);
    let text = fs::read_to_string(&first)?;
    assert!(text.ends_with('\n'));
    assert!(!text.contains('\r'));
    assert!(text.starts_with("{\"checksum\":"));
    let format_position = text
        .find("\"format_version\"")
        .ok_or("missing format key")?;
    let kind_position = text.find("\"kind\"").ok_or("missing kind key")?;
    let payload_position = text.find("\"payload\"").ok_or("missing payload key")?;
    assert!(format_position < kind_position && kind_position < payload_position);
    assert!(text.contains(&format!("\"kind\":\"{BUNDLE_KIND}\"")));
    assert!(text.contains(&format!("\"format_version\":{BUNDLE_FORMAT_VERSION}")));
    assert!(!text.contains(clone.to_str().ok_or("non-UTF8 clone")?));
    for excluded in [
        "local_bindings",
        "managed_bundle_paths",
        "operations",
        "embedding",
        "parser_cache",
        "raw_tool_traffic",
        "full_chat_transcript",
        "VOLICORD_HOME",
    ] {
        assert!(
            !text.contains(excluded),
            "excluded class leaked: {excluded}"
        );
    }
    Ok(())
}

#[test]
fn import_preserves_ids_is_idempotent_and_allows_explicit_another_path_binding(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let clone_a = root.path().join("clone-a");
    let clone_b = root.path().join("clone-b");
    fs::create_dir(&clone_a)?;
    fs::create_dir(&clone_b)?;
    let source_path = root.path().join("source.sqlite3");
    let mut source = test_store(&source_path, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11])?;
    let fixture = populate(&mut source, &clone_a, "identity survives")?;
    source.correct_context_item(
        operation(10),
        fixture.project.id,
        fixture.item.id,
        ContextItemCorrectionDraft {
            expected_revision: 1,
            corrected_statement: "identity  survives".to_owned(),
            kind: CorrectionKind::Formatting,
            user_authorization_source_id: fixture.authorization.id,
        },
    )?;
    let superseding = source
        .supersede_decision(
            operation(11),
            fixture.project.id,
            DecisionSupersessionDraft {
                expected_project_revision: 1,
                previous_decision_id: fixture.decision.id,
                user_turn_source: UserTurnSource::Create(user_turn("portable-supersession")),
                choice: DecisionChoice::Delegation {
                    delegate_to: "future implementation".to_owned(),
                },
                user_rationale: Some("Delegate the next format choice".to_owned()),
                applicability: ApplicabilityScope::default(),
                assumptions: vec![],
                revisit_triggers: vec![],
            },
        )?
        .value;
    let bundle = root.path().join("context.json");
    source.export_bundle(fixture.project.id, &bundle)?;
    drop(source);
    fs::remove_dir_all(&clone_a)?;

    let destination_path = root.path().join("destination.sqlite3");
    let mut destination = test_store(&destination_path, &[20])?;
    let imported = destination.import_bundle(operation(30), &bundle)?;
    assert_eq!(imported.value.status, BundleImportStatus::Imported);
    assert_eq!(imported.value.project_id, fixture.project.id);
    assert_eq!(
        destination.get_project(fixture.project.id)?.id,
        fixture.project.id
    );
    let imported_source = destination.get_source(fixture.project.id, fixture.repository.id)?;
    assert_eq!(imported_source.id, fixture.repository.id);
    assert_eq!(imported_source.availability, Availability::Unavailable);
    assert_eq!(
        destination
            .get_context_item(fixture.project.id, fixture.item.id)?
            .statement,
        "identity  survives"
    );
    assert_eq!(
        destination
            .get_context_item_history(fixture.project.id, fixture.item.id)?
            .len(),
        2
    );
    assert_eq!(
        destination
            .get_decision(fixture.project.id, fixture.decision.id)?
            .id,
        fixture.decision.id
    );
    assert_eq!(
        destination
            .get_current_decision(fixture.project.id, fixture.decision.question_id)?
            .decision
            .id,
        superseding.id
    );
    assert_eq!(
        destination
            .get_canonical_relation(
                fixture.project.id,
                CanonicalRecordId::Decision(superseding.id),
                CanonicalRelationKind::Supersedes,
                CanonicalRecordId::Decision(fixture.decision.id),
            )?
            .to,
        CanonicalRecordId::Decision(fixture.decision.id)
    );
    assert_eq!(
        destination
            .get_checkpoint(fixture.project.id, fixture.checkpoint.id)?
            .id,
        fixture.checkpoint.id
    );
    assert_eq!(
        destination
            .get_local_binding(fixture.project.id)
            .err()
            .ok_or("binding must not import")?
            .kind(),
        ErrorKind::NotFound
    );
    let repeated = destination.import_bundle(operation(31), &bundle)?;
    assert_eq!(repeated.value.status, BundleImportStatus::AlreadyPresent);
    assert!(!repeated.replayed);
    assert!(destination.import_bundle(operation(31), &bundle)?.replayed);
    let binding = destination
        .bind_clone(
            operation(32),
            fixture.project.id,
            None,
            &clone_b,
            Availability::Available,
        )?
        .value;
    assert_eq!(binding.absolute_path, clone_b);
    drop(destination);
    let mut reopened = test_store(&destination_path, &[])?;
    assert!(reopened.import_bundle(operation(30), &bundle)?.replayed);
    assert_eq!(
        reopened
            .get_decision(fixture.project.id, fixture.decision.id)?
            .id,
        fixture.decision.id
    );
    Ok(())
}

#[test]
fn corruption_and_newer_version_fail_before_any_mutation() -> Result<(), Box<dyn std::error::Error>>
{
    let root = tempdir()?;
    let clone = root.path().join("clone");
    fs::create_dir(&clone)?;
    let mut source = test_store(
        &root.path().join("source.sqlite3"),
        &[1, 2, 3, 4, 5, 6, 7, 8, 9],
    )?;
    let fixture = populate(&mut source, &clone, "checksum basis")?;
    let bundle = root.path().join("context.json");
    source.export_bundle(fixture.project.id, &bundle)?;
    let original = fs::read(&bundle)?;

    let corrupt = root.path().join("corrupt.json");
    let mut corrupt_bytes = original.clone();
    let position = corrupt_bytes
        .windows("checksum basis".len())
        .position(|window| window == b"checksum basis")
        .ok_or("content missing")?;
    corrupt_bytes[position] = b'X';
    fs::write(&corrupt, corrupt_bytes)?;
    let destination_path = root.path().join("destination.sqlite3");
    let mut destination = test_store(&destination_path, &[])?;
    assert_eq!(
        destination
            .import_bundle(operation(40), &corrupt)
            .err()
            .ok_or("expected checksum failure")?
            .kind(),
        ErrorKind::IntegrityFailure
    );
    assert_eq!(
        destination
            .get_project(fixture.project.id)
            .err()
            .ok_or("corrupt import mutated")?
            .kind(),
        ErrorKind::NotFound
    );

    let newer = root.path().join("newer.json");
    let text = String::from_utf8(original)?;
    fs::write(
        &newer,
        text.replacen("\"format_version\":2", "\"format_version\":3", 1),
    )?;
    assert_eq!(
        destination
            .import_bundle(operation(41), &newer)
            .err()
            .ok_or("expected version failure")?
            .kind(),
        ErrorKind::UnsupportedVersion
    );
    assert_eq!(
        destination
            .get_project(fixture.project.id)
            .err()
            .ok_or("newer import mutated")?
            .kind(),
        ErrorKind::NotFound
    );
    Ok(())
}

#[test]
fn malformed_relation_and_interrupted_import_roll_back_fully(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let clone = root.path().join("clone");
    fs::create_dir(&clone)?;
    let source_path = root.path().join("source.sqlite3");
    let mut source = test_store(&source_path, &[1, 2, 3, 4, 5, 6, 7, 8, 9])?;
    let fixture = populate(&mut source, &clone, "rollback basis")?;
    drop(source);
    let connection = Connection::open(&source_path)?;
    connection.execute(
        "INSERT INTO canonical_relations(project_id, from_kind, from_id, relation_kind, to_kind, to_id, recorded_at) VALUES (?1, 'context_item', ?2, 'contradicts', 'context_item', ?3, 1)",
        params![fixture.project.id.as_bytes().as_slice(), fixture.item.id.as_bytes().as_slice(), [99_u8; 16].as_slice()],
    )?;
    drop(connection);
    let mut source = test_store(&source_path, &[])?;
    let malformed = root.path().join("malformed.json");
    source.export_bundle(fixture.project.id, &malformed)?;
    let mut destination = test_store(&root.path().join("malformed-destination.sqlite3"), &[])?;
    assert_eq!(
        destination
            .import_bundle(operation(50), &malformed)
            .err()
            .ok_or("expected malformed relation")?
            .kind(),
        ErrorKind::CorruptState
    );
    assert_eq!(
        destination
            .get_project(fixture.project.id)
            .err()
            .ok_or("malformed import mutated")?
            .kind(),
        ErrorKind::NotFound
    );

    let connection = Connection::open(&source_path)?;
    connection.execute(
        "DELETE FROM canonical_relations WHERE to_id = ?1",
        [[99_u8; 16].as_slice()],
    )?;
    drop(connection);
    let source = test_store(&source_path, &[])?;
    let valid = root.path().join("valid.json");
    let mut source = source;
    source.export_bundle(fixture.project.id, &valid)?;
    let interrupted_path = root.path().join("interrupted.sqlite3");
    let mut interrupted = test_store(&interrupted_path, &[55])?;
    let prior = interrupted
        .create_project(operation(52), "Prior committed Project")?
        .value;
    drop(interrupted);
    let connection = Connection::open(&interrupted_path)?;
    connection.execute_batch("CREATE TRIGGER interrupt_import BEFORE INSERT ON context_items BEGIN SELECT RAISE(ABORT, 'interrupted'); END;")?;
    drop(connection);
    let mut interrupted = test_store(&interrupted_path, &[])?;
    assert_eq!(
        interrupted
            .import_bundle(operation(51), &valid)
            .err()
            .ok_or("expected import interruption")?
            .kind(),
        ErrorKind::TransactionFailure
    );
    assert_eq!(
        interrupted
            .get_project(fixture.project.id)
            .err()
            .ok_or("interrupted import mutated")?
            .kind(),
        ErrorKind::NotFound
    );
    assert_eq!(
        interrupted.get_project(prior.id)?.display_name,
        "Prior committed Project"
    );
    Ok(())
}

#[test]
fn publication_interruption_keeps_previous_file_and_forgetting_refreshes_managed_bundle(
) -> Result<(), Box<dyn std::error::Error>> {
    const SECRET: &str = "portable-forget-secret-44321";
    let root = tempdir()?;
    let clone = root.path().join("clone");
    fs::create_dir(&clone)?;
    let mut store = test_store(
        &root.path().join("source.sqlite3"),
        &[1, 2, 3, 4, 5, 6, 7, 8, 9],
    )?;
    let fixture = populate(&mut store, &clone, SECRET)?;
    let bundle = root.path().join("context.json");
    store.export_bundle(fixture.project.id, &bundle)?;
    let previous = fs::read(&bundle)?;
    assert!(previous
        .windows(SECRET.len())
        .any(|window| window == SECRET.as_bytes()));
    let temporary = root.path().join(".context.json.volicord-context.tmp");
    fs::create_dir(&temporary)?;
    assert_eq!(
        store
            .export_bundle(fixture.project.id, &bundle)
            .err()
            .ok_or("expected publication interruption")?
            .kind(),
        ErrorKind::StorageUnavailable
    );
    assert_eq!(fs::read(&bundle)?, previous);
    assert_eq!(
        store
            .forget_context_item(
                operation(60),
                fixture.project.id,
                fixture.item.id,
                fixture.authorization.id,
            )
            .err()
            .ok_or("expected managed refresh repair requirement")?
            .kind(),
        ErrorKind::RepairRequired
    );
    assert_eq!(
        store
            .get_context_item(fixture.project.id, fixture.item.id)
            .err()
            .ok_or("forgetting should have committed")?
            .kind(),
        ErrorKind::NotFound
    );
    assert_eq!(fs::read(&bundle)?, previous);
    fs::remove_dir(&temporary)?;
    assert!(
        store
            .forget_context_item(
                operation(60),
                fixture.project.id,
                fixture.item.id,
                fixture.authorization.id,
            )?
            .replayed
    );
    let refreshed = fs::read(&bundle)?;
    assert!(!refreshed
        .windows(SECRET.len())
        .any(|window| window == SECRET.as_bytes()));
    assert!(String::from_utf8(refreshed)?.contains("tombstones"));
    Ok(())
}
