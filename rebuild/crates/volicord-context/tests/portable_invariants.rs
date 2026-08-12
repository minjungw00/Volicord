use rusqlite::Connection;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use volicord_context::{
    AgentRecommendation, ApplicabilityScope, Availability, CanonicalReadOptions, CheckpointDraft,
    CheckpointKind, CommandOutcome, CommandTermination, ContextItemCorrectionDraft,
    ContextItemDraft, ContextItemRole, CorrectionKind, DecisionChoice, DecisionCorrectionDraft,
    DecisionSupersessionDraft, DeterministicIdGenerator, ErrorKind, ExplicitQuestionResponse,
    FixedClock, MergeResolution, MergeResolutionMode, OperationId, Principal, PrincipalKind,
    ProjectId, QuestionAlternative, QuestionDraft, QuestionId, QuestionResponseDraft, SourceDraft,
    SourceId, SourcePayload, StatementProvenanceRole, Store, TimestampMicros, UserAcceptanceFact,
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

fn user_turn(turn: &str) -> SourceDraft {
    SourceDraft {
        expected_project_revision: 1,
        payload: SourcePayload::CurrentHostUserTurn {
            host: "codex".to_owned(),
            session: "portable-invariant-session".to_owned(),
            turn: turn.to_owned(),
        },
        actor: Principal {
            kind: PrincipalKind::User,
            identity: "project-owner".to_owned(),
        },
        observer: None,
        availability: Availability::Available,
    }
}

struct Fixture {
    project_id: ProjectId,
    question_id: QuestionId,
    decision_id: volicord_context::DecisionId,
    file_source_id: SourceId,
    authorization_id: SourceId,
    active_bundle: PathBuf,
    forgotten_bundle: PathBuf,
}

fn fixture(root: &Path) -> Result<Fixture, Box<dyn std::error::Error>> {
    let database = root.join("origin.sqlite3");
    let active_bundle = root.join("active.json");
    let forgotten_bundle = root.join("forgotten.json");
    let mut value = store(&database, &[1, 2, 3, 4, 5])?;
    let project = value
        .create_project(operation(1), "Portable invariant")?
        .value;
    let source = value
        .record_source(
            operation(2),
            project.id,
            SourceDraft {
                expected_project_revision: 1,
                payload: SourcePayload::File {
                    locator: "src/lib.rs".to_owned(),
                    snapshot: "portable-invariant-base".to_owned(),
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
    let authorization = value
        .record_source(operation(3), project.id, user_turn("authorization"))?
        .value;
    let question = value
        .create_question(
            operation(4),
            project.id,
            QuestionDraft {
                expected_project_revision: 1,
                prompt_basis: "Choose a portable boundary".to_owned(),
                source_basis: vec![source.id],
                dependencies: vec![],
                alternatives: vec![QuestionAlternative {
                    key: "central".to_owned(),
                    label: "Central validation".to_owned(),
                    consequence: "Reject inconsistent portable state".to_owned(),
                }],
                recommendation: AgentRecommendation {
                    alternative_key: Some("central".to_owned()),
                    rationale: "One semantic boundary protects every consumer".to_owned(),
                    source_basis: vec![source.id],
                },
                trade_offs: vec![],
                uncertainty: vec![],
                material_scope: vec!["portable context".to_owned()],
            },
        )?
        .value;
    let decision = value
        .record_question_response(
            operation(5),
            project.id,
            QuestionResponseDraft {
                expected_project_revision: 1,
                question_id: question.id,
                question_revision: 1,
                user_turn_source: UserTurnSource::Existing(authorization.id),
                displayed_alternative_keys: vec!["central".to_owned()],
                displayed_recommendation_key: Some("central".to_owned()),
                response: ExplicitQuestionResponse::Choice {
                    alternative_key: "central".to_owned(),
                    user_rationale: Some("Keep independently owned rationale".to_owned()),
                },
                applicability: ApplicabilityScope {
                    paths: vec!["rebuild/".to_owned()],
                    components: vec!["context".to_owned()],
                    work_contexts: vec!["portable validation".to_owned()],
                },
                assumptions: vec!["current format".to_owned()],
                revisit_triggers: vec!["format evolution".to_owned()],
            },
        )?
        .value
        .decision
        .ok_or("Decision missing")?;
    value.export_bundle(project.id, &active_bundle)?;
    let active_bytes = fs::read(&active_bundle)?;
    value.forget_question(operation(6), project.id, question.id, authorization.id)?;
    value.export_bundle(project.id, &forgotten_bundle)?;
    fs::write(&active_bundle, active_bytes)?;
    Ok(Fixture {
        project_id: project.id,
        question_id: question.id,
        decision_id: decision.id,
        file_source_id: source.id,
        authorization_id: authorization.id,
        active_bundle,
        forgotten_bundle,
    })
}

fn semantic_fixture(root: &Path) -> Result<Fixture, Box<dyn std::error::Error>> {
    let mut fixture = fixture(root)?;
    let database = root.join("semantic-origin.sqlite3");
    let active_bundle = root.join("semantic-active.json");
    let mut value = store(&database, &[0x60, 0x61, 0x62])?;
    value.import_bundle(operation(150), &fixture.active_bundle)?;
    let command = value
        .record_source(
            operation(151),
            fixture.project_id,
            SourceDraft {
                expected_project_revision: 1,
                payload: SourcePayload::CommandExecution {
                    command_label: "cargo test -p volicord-context".to_owned(),
                    outcome: CommandOutcome {
                        exit_code: Some(0),
                        termination: CommandTermination::Exited,
                    },
                },
                actor: Principal {
                    kind: PrincipalKind::Command,
                    identity: "test-observer".to_owned(),
                },
                observer: Some(Principal {
                    kind: PrincipalKind::Agent,
                    identity: "codex".to_owned(),
                }),
                availability: Availability::Available,
            },
        )?
        .value;
    let context = value
        .record_context_item(
            operation(152),
            fixture.project_id,
            ContextItemDraft {
                expected_project_revision: 1,
                role: ContextItemRole::Fact,
                statement: "portable semantic fact".to_owned(),
                provenance_role: StatementProvenanceRole::Observed,
                author: Principal {
                    kind: PrincipalKind::Agent,
                    identity: "fact-observer".to_owned(),
                },
                source_basis: vec![fixture.file_source_id],
                applicability: ApplicabilityScope {
                    paths: vec!["rebuild/".to_owned()],
                    components: vec!["context".to_owned()],
                    work_contexts: vec!["semantic admission".to_owned()],
                },
            },
        )?
        .value;
    value.correct_context_item(
        operation(153),
        fixture.project_id,
        context.id,
        ContextItemCorrectionDraft {
            expected_revision: 1,
            corrected_statement: "portable-semantic fact".to_owned(),
            kind: CorrectionKind::Formatting,
            user_authorization_source_id: fixture.authorization_id,
        },
    )?;
    value.record_checkpoint(
        operation(154),
        fixture.project_id,
        CheckpointDraft {
            expected_project_revision: 1,
            kind: CheckpointKind::Completion,
            goal: "Close portable semantic parity".to_owned(),
            work_state: WorkState::Completed,
            state_change: Some("Central admission validates typed semantics".to_owned()),
            source_basis: vec![fixture.file_source_id],
            changed_source_basis: vec![fixture.file_source_id],
            changed_paths: vec!["rebuild/crates/volicord-context".to_owned()],
            applied_decisions: vec![],
            verification: vec![VerificationFact {
                state: VerificationState::Passed,
                source_id: Some(command.id),
                outcome: Some("volicord-context tests passed".to_owned()),
            }],
            user_review: UserReviewFact {
                state: UserReviewState::Reviewed,
                source_id: Some(fixture.authorization_id),
            },
            user_acceptance: UserAcceptanceFact {
                state: UserAcceptanceState::Accepted,
                source_id: Some(fixture.authorization_id),
            },
            known_limits: vec!["Independent final audit remains".to_owned()],
            non_goals: vec!["No format-version change".to_owned()],
            open_questions: vec![],
            next_step: "Run focused validation".to_owned(),
            handoff_to: None,
        },
    )?;
    value.export_bundle(fixture.project_id, &active_bundle)?;
    fixture.active_bundle = active_bundle;
    Ok(fixture)
}

fn read_json(path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn table<'a>(document: &'a Value, name: &str) -> &'a Value {
    document["payload"]["tables"]
        .as_array()
        .and_then(|tables| tables.iter().find(|table| table["name"] == name))
        .unwrap_or_else(|| panic!("missing table {name}"))
}

fn table_mut<'a>(document: &'a mut Value, name: &str) -> &'a mut Value {
    document["payload"]["tables"]
        .as_array_mut()
        .and_then(|tables| tables.iter_mut().find(|table| table["name"] == name))
        .unwrap_or_else(|| panic!("missing table {name}"))
}

fn column(table: &Value, name: &str) -> usize {
    table["columns"]
        .as_array()
        .and_then(|columns| columns.iter().position(|value| value == name))
        .unwrap_or_else(|| panic!("missing column {name}"))
}

fn copy_presentation(target: &mut Value, source: &Value, table_name: &str) {
    let names = [
        "displayed_alternatives",
        "recommendation_key",
        "recommendation_rationale",
        "recommendation_sources",
    ];
    let source_table = table(source, table_name);
    let copied = names.map(|name| {
        let index = column(source_table, name);
        source_table["rows"][0][index].clone()
    });
    let target_table = table_mut(target, table_name);
    for (name, value) in names.into_iter().zip(copied) {
        let index = column(target_table, name);
        target_table["rows"][0][index] = value;
    }
}

fn remove_review_due(document: &mut Value) {
    table_mut(document, "review_due")["rows"] = Value::Array(Vec::new());
}

fn push_tombstone(document: &mut Value, record_kind: &str, record_id: Value) {
    let project_id = table(document, "projects")["rows"][0][0].clone();
    table_mut(document, "tombstones")["rows"]
        .as_array_mut()
        .unwrap_or_else(|| panic!("tombstone rows missing"))
        .push(serde_json::json!([
            project_id,
            {"type": "text", "value": record_kind},
            record_id,
            {"type": "integer", "value": 1_790_000_000_000_000_i64}
        ]));
}

fn set_cell(document: &mut Value, table_name: &str, row: usize, name: &str, value: Value) {
    let target = table_mut(document, table_name);
    let index = column(target, name);
    target["rows"][row][index] = value;
}

fn row_with_text(document: &Value, table_name: &str, name: &str, value: &str) -> usize {
    let target = table(document, table_name);
    let index = column(target, name);
    target["rows"]
        .as_array()
        .and_then(|rows| rows.iter().position(|row| row[index]["value"] == value))
        .unwrap_or_else(|| panic!("missing {table_name} row with {name}={value}"))
}

fn portable_strings(values: &[&str]) -> Value {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(values.len() as u64).to_be_bytes());
    for value in values {
        bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
        bytes.extend_from_slice(value.as_bytes());
    }
    serde_json::json!({
        "type": "bytes",
        "value": bytes.iter().map(|byte| format!("{byte:02x}")).collect::<String>()
    })
}

fn copy_cell(
    document: &mut Value,
    source_table: &str,
    source_row: usize,
    source_name: &str,
    target_table: &str,
    target_row: usize,
    target_name: &str,
) {
    let value = {
        let source = table(document, source_table);
        source["rows"][source_row][column(source, source_name)].clone()
    };
    set_cell(document, target_table, target_row, target_name, value);
}

fn portable_id(value: u8) -> Value {
    serde_json::json!({"type": "bytes", "value": format!("{value:02x}").repeat(16)})
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn write_recomputed(path: &Path, mut document: Value) -> Result<(), Box<dyn std::error::Error>> {
    let state = serde_json::json!({
        "project_id": document["payload"]["project_id"].clone(),
        "tables": document["payload"]["tables"].clone(),
    });
    let history_basis = sha256_hex(&serde_json::to_vec(&state)?);
    document["payload"]["lineage"]["history_basis"] = Value::String(history_basis.clone());
    document["payload"]["lineage"]["common_base_basis"] = Value::String(history_basis);
    let checksum = sha256_hex(&serde_json::to_vec(&document["payload"])?);
    document["checksum"] = Value::String(checksum);
    let mut bytes = serde_json::to_vec(&document)?;
    bytes.push(b'\n');
    fs::write(path, bytes)?;
    Ok(())
}

type OperationState = (Vec<u8>, String, Vec<u8>, String, String, Vec<u8>, i64, i64);
type LineageState = (Vec<u8>, String);

fn operation_state(path: &Path) -> Result<Vec<OperationState>, Box<dyn std::error::Error>> {
    let connection = Connection::open(path)?;
    let mut statement = connection.prepare(
        "SELECT operation_id, operation_kind, input_basis, replay_state, result_kind,
                result_id, result_revision, committed_at
         FROM operations ORDER BY operation_id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
            row.get(7)?,
        ))
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn lineage_state(path: &Path) -> Result<Vec<LineageState>, Box<dyn std::error::Error>> {
    let connection = Connection::open(path)?;
    let mut statement = connection
        .prepare("SELECT project_id, common_base_basis FROM bundle_lineage ORDER BY project_id")?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn assert_import_rejected_without_mutation(
    root: &Path,
    fixture: &Fixture,
    invalid: &Path,
    operation_byte: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let database = root.join(format!("destination-{operation_byte}.sqlite3"));
    let managed = root.join(format!("managed-{operation_byte}.json"));
    let clone = root.join(format!("clone-{operation_byte}"));
    fs::create_dir(&clone)?;
    let mut destination = store(&database, &[21])?;
    destination.import_bundle(operation(20), &fixture.active_bundle)?;
    destination.bind_clone(
        operation(21),
        fixture.project_id,
        None,
        &clone,
        Availability::Available,
    )?;
    destination.export_bundle(fixture.project_id, &managed)?;
    let before_read =
        destination.read_canonical_basis(fixture.project_id, CanonicalReadOptions::default())?;
    let before_export = fs::read(&managed)?;
    let before_operations = operation_state(&database)?;
    let before_lineage = lineage_state(&database)?;
    let before_binding = destination.get_local_binding(fixture.project_id)?;
    assert_eq!(
        destination
            .import_bundle(operation(operation_byte), invalid)
            .err()
            .ok_or("invalid portable state imported")?
            .kind(),
        ErrorKind::CorruptState
    );
    assert_eq!(
        destination.read_canonical_basis(fixture.project_id, CanonicalReadOptions::default(),)?,
        before_read
    );
    let repeat = root.join(format!("repeat-{operation_byte}.json"));
    destination.export_bundle(fixture.project_id, &repeat)?;
    assert_eq!(fs::read(&repeat)?, before_export);
    assert_eq!(fs::read(&managed)?, before_export);
    assert_eq!(operation_state(&database)?, before_operations);
    assert_eq!(lineage_state(&database)?, before_lineage);
    assert_eq!(
        destination.get_local_binding(fixture.project_id)?,
        before_binding
    );
    drop(destination);
    let reopened = store(&database, &[])?;
    assert_eq!(
        reopened.read_canonical_basis(fixture.project_id, CanonicalReadOptions::default(),)?,
        before_read
    );
    assert_eq!(
        reopened.get_local_binding(fixture.project_id)?,
        before_binding
    );
    Ok(())
}

fn assert_clean_import_rejected(
    root: &Path,
    fixture: &Fixture,
    label: &str,
    document: Value,
    operation_byte: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let bundle = root.join(format!("{label}.json"));
    write_recomputed(&bundle, document)?;
    let mut destination = store(&root.join(format!("{label}.sqlite3")), &[])?;
    assert_eq!(
        destination
            .import_bundle(operation(operation_byte), &bundle)
            .err()
            .ok_or("crafted Decision state passed portable admission")?
            .kind(),
        ErrorKind::CorruptState
    );
    assert_eq!(
        destination
            .get_question(fixture.project_id, fixture.question_id)
            .err()
            .ok_or("crafted import partially mutated canonical state")?
            .kind(),
        ErrorKind::NotFound
    );
    Ok(())
}

fn assert_portable_write_boundaries_reject(
    root: &Path,
    fixture: &Fixture,
    label: &str,
    document: Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let case_root = root.join(label);
    fs::create_dir(&case_root)?;
    let bundle = case_root.join("mutated.json");
    write_recomputed(&bundle, document)?;

    assert_import_rejected_without_mutation(&case_root, fixture, &bundle, 200)?;
    assert_explicit_merge_rejected(&case_root, fixture, &bundle, false, false, 201)?;
    Ok(())
}

#[test]
fn command_generated_state_round_trips_with_semantic_and_deterministic_equivalence(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let first_database = root.path().join("equivalence-first.sqlite3");
    let first_export = root.path().join("equivalence-first.json");
    let after_binding = root.path().join("equivalence-after-binding.json");
    let clone = root.path().join("equivalence-clone");
    fs::create_dir(&clone)?;

    let mut first = store(&first_database, &[220])?;
    first.import_bundle(operation(210), &fixture.active_bundle)?;
    let expected =
        first.read_canonical_basis(fixture.project_id, CanonicalReadOptions::default())?;
    first.export_bundle(fixture.project_id, &first_export)?;
    first.bind_clone(
        operation(211),
        fixture.project_id,
        None,
        &clone,
        Availability::Available,
    )?;
    first.export_bundle(fixture.project_id, &after_binding)?;
    assert_eq!(fs::read(&first_export)?, fs::read(&after_binding)?);
    drop(first);

    let reopened = store(&first_database, &[])?;
    assert_eq!(
        reopened.read_canonical_basis(fixture.project_id, CanonicalReadOptions::default())?,
        expected
    );
    assert_eq!(
        reopened
            .get_local_binding(fixture.project_id)?
            .absolute_path,
        clone
    );

    let second_export = root.path().join("equivalence-second.json");
    let mut second = store(&root.path().join("equivalence-second.sqlite3"), &[])?;
    second.import_bundle(operation(212), &first_export)?;
    assert_eq!(
        second.read_canonical_basis(fixture.project_id, CanonicalReadOptions::default())?,
        expected
    );
    assert!(second.get_local_binding(fixture.project_id).is_err());
    second.export_bundle(fixture.project_id, &second_export)?;
    assert_eq!(fs::read(&first_export)?, fs::read(&second_export)?);
    Ok(())
}

#[test]
fn valid_direct_decision_lineages_pass_every_portable_forgetting_boundary(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let database = root.path().join("valid-lineages.sqlite3");
    let mut origin = store(&database, &[6, 7])?;
    origin.import_bundle(operation(80), &fixture.active_bundle)?;
    let forgetting_authority = origin
        .record_source(
            operation(81),
            fixture.project_id,
            user_turn("forget-authority"),
        )?
        .value;
    origin.correct_decision(
        operation(82),
        fixture.project_id,
        fixture.decision_id,
        DecisionCorrectionDraft {
            expected_revision: 1,
            corrected_user_rationale: Some("Keep independently-owned rationale".to_owned()),
            kind: volicord_context::CorrectionKind::Typography,
            user_authorization_source_id: fixture.authorization_id,
        },
    )?;
    let superseding = origin
        .supersede_decision(
            operation(83),
            fixture.project_id,
            DecisionSupersessionDraft {
                expected_project_revision: 1,
                previous_decision_id: fixture.decision_id,
                user_turn_source: UserTurnSource::Existing(fixture.authorization_id),
                choice: DecisionChoice::Delegation {
                    delegate_to: "implementation owner".to_owned(),
                },
                user_rationale: Some("Delegate the follow-up".to_owned()),
                applicability: ApplicabilityScope::default(),
                assumptions: vec!["same Question basis".to_owned()],
                revisit_triggers: vec!["owner unavailable".to_owned()],
            },
        )?
        .value;

    let lineage = root.path().join("valid-lineage.json");
    origin.export_bundle(fixture.project_id, &lineage)?;
    let lineage_document = read_json(&lineage)?;
    let original_witness =
        table(&lineage_document, "question_decision_history_witnesses")["rows"][0].clone();
    let witness_table = table(&lineage_document, "question_decision_history_witnesses");
    assert_eq!(
        original_witness[column(witness_table, "root_decision_id")]["value"],
        fixture.decision_id.to_string()
    );
    assert_eq!(
        original_witness[column(witness_table, "terminal_outcome")]["value"],
        "answered"
    );
    let mut imported = store(&root.path().join("valid-lineage-import.sqlite3"), &[])?;
    imported.import_bundle(operation(84), &lineage)?;
    assert_eq!(
        imported
            .get_decision(fixture.project_id, fixture.decision_id)?
            .revision,
        2
    );
    assert_eq!(
        imported
            .get_decision(fixture.project_id, superseding.id)?
            .choice,
        DecisionChoice::Delegation {
            delegate_to: "implementation owner".to_owned()
        }
    );

    origin.forget_source(
        operation(85),
        fixture.project_id,
        fixture.authorization_id,
        forgetting_authority.id,
    )?;
    let source_forgotten = root.path().join("valid-source-forgotten.json");
    origin.export_bundle(fixture.project_id, &source_forgotten)?;
    assert_eq!(
        table(
            &read_json(&source_forgotten)?,
            "question_decision_history_witnesses"
        )["rows"][0],
        original_witness
    );
    let mut source_import = store(&root.path().join("source-forgotten-import.sqlite3"), &[])?;
    source_import.import_bundle(operation(86), &source_forgotten)?;
    assert_eq!(
        source_import
            .get_decision(fixture.project_id, superseding.id)?
            .choice,
        DecisionChoice::Delegation {
            delegate_to: "implementation owner".to_owned()
        }
    );

    origin.forget_question(
        operation(87),
        fixture.project_id,
        fixture.question_id,
        forgetting_authority.id,
    )?;
    let question_forgotten = root.path().join("valid-question-forgotten.json");
    origin.export_bundle(fixture.project_id, &question_forgotten)?;
    assert_eq!(
        table(
            &read_json(&question_forgotten)?,
            "question_decision_history_witnesses"
        )["rows"][0],
        original_witness
    );
    let mut question_import = store(&root.path().join("question-forgotten-import.sqlite3"), &[])?;
    question_import.import_bundle(operation(88), &question_forgotten)?;
    assert!(question_import
        .get_decision_lifecycle(fixture.project_id, fixture.decision_id)?
        .review_due
        .is_some());
    assert!(question_import
        .get_decision_lifecycle(fixture.project_id, superseding.id)?
        .review_due
        .is_some());

    origin.forget_decision(
        operation(89),
        fixture.project_id,
        fixture.decision_id,
        forgetting_authority.id,
    )?;
    let root_forgotten = root.path().join("valid-root-forgotten-lineage.json");
    origin.export_bundle(fixture.project_id, &root_forgotten)?;
    assert_eq!(
        table(
            &read_json(&root_forgotten)?,
            "question_decision_history_witnesses"
        )["rows"][0],
        original_witness
    );
    let mut root_import = store(&root.path().join("root-forgotten-import.sqlite3"), &[])?;
    root_import.import_bundle(operation(90), &root_forgotten)?;
    assert_eq!(
        root_import
            .get_decision(fixture.project_id, superseding.id)?
            .choice,
        DecisionChoice::Delegation {
            delegate_to: "implementation owner".to_owned()
        }
    );
    assert_eq!(
        root_import
            .get_tombstone(
                fixture.project_id,
                volicord_context::CanonicalRecordId::Decision(fixture.decision_id),
            )?
            .record,
        volicord_context::CanonicalRecordId::Decision(fixture.decision_id)
    );
    Ok(())
}

#[test]
fn direct_decision_admission_rejects_the_same_representative_semantics(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut value = store(
        &root.path().join("direct-parity.sqlite3"),
        &[1, 2, 3, 4, 5, 6],
    )?;
    let project = value.create_project(operation(110), "Direct parity")?.value;
    let file = value
        .record_source(
            operation(111),
            project.id,
            SourceDraft {
                expected_project_revision: 1,
                payload: SourcePayload::File {
                    locator: "src/lib.rs".to_owned(),
                    snapshot: "direct-parity".to_owned(),
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
    let user = value
        .record_source(operation(112), project.id, user_turn("direct-parity"))?
        .value;
    let agent = value
        .record_source(
            operation(113),
            project.id,
            SourceDraft {
                expected_project_revision: 1,
                payload: SourcePayload::CurrentHostUserTurn {
                    host: "codex".to_owned(),
                    session: "direct-parity".to_owned(),
                    turn: "agent-turn".to_owned(),
                },
                actor: Principal {
                    kind: PrincipalKind::Agent,
                    identity: "agent".to_owned(),
                },
                observer: None,
                availability: Availability::Available,
            },
        )?
        .value;
    let question = value
        .create_question(
            operation(114),
            project.id,
            QuestionDraft {
                expected_project_revision: 1,
                prompt_basis: "Choose the parity boundary".to_owned(),
                source_basis: vec![file.id],
                dependencies: vec![],
                alternatives: vec![QuestionAlternative {
                    key: "central".to_owned(),
                    label: "Central".to_owned(),
                    consequence: "One invariant boundary".to_owned(),
                }],
                recommendation: AgentRecommendation {
                    alternative_key: Some("central".to_owned()),
                    rationale: "Keep admission aligned".to_owned(),
                    source_basis: vec![file.id],
                },
                trade_offs: vec![],
                uncertainty: vec![],
                material_scope: vec!["portable Decision".to_owned()],
            },
        )?
        .value;
    let draft = |source_id, revision, alternative: &str| QuestionResponseDraft {
        expected_project_revision: 1,
        question_id: question.id,
        question_revision: revision,
        user_turn_source: UserTurnSource::Existing(source_id),
        displayed_alternative_keys: vec!["central".to_owned()],
        displayed_recommendation_key: Some("central".to_owned()),
        response: ExplicitQuestionResponse::Choice {
            alternative_key: alternative.to_owned(),
            user_rationale: Some("Explicit choice".to_owned()),
        },
        applicability: ApplicabilityScope::default(),
        assumptions: vec![],
        revisit_triggers: vec![],
    };
    assert_eq!(
        value
            .record_question_response(operation(115), project.id, draft(file.id, 1, "central"))
            .err()
            .ok_or("file Source authorized a direct Decision")?
            .kind(),
        ErrorKind::InvalidInput
    );
    assert_eq!(
        value
            .record_question_response(operation(116), project.id, draft(agent.id, 1, "central"))
            .err()
            .ok_or("agent Source authorized a direct Decision")?
            .kind(),
        ErrorKind::InvalidInput
    );
    assert_eq!(
        value
            .record_question_response(operation(117), project.id, draft(user.id, 99, "central"))
            .err()
            .ok_or("nonexistent Question revision created a direct Decision")?
            .kind(),
        ErrorKind::StaleBasis
    );
    assert_eq!(
        value
            .record_question_response(operation(118), project.id, draft(user.id, 1, "forged"))
            .err()
            .ok_or("undisplayed alternative created a direct Decision")?
            .kind(),
        ErrorKind::InvalidInput
    );
    value.record_question_response(operation(119), project.id, draft(user.id, 1, "central"))?;
    value.export_bundle(project.id, root.path().join("direct-parity.json"))?;
    Ok(())
}

#[test]
fn portable_rejects_file_source_as_decision_authority() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let mut document = read_json(&fixture.active_bundle)?;
    copy_cell(
        &mut document,
        "sources",
        0,
        "id",
        "decisions",
        0,
        "user_turn_source_id",
    );
    copy_cell(
        &mut document,
        "sources",
        0,
        "id",
        "decision_revisions",
        0,
        "user_turn_source_id",
    );
    copy_cell(
        &mut document,
        "sources",
        0,
        "id",
        "question_response_sources",
        0,
        "source_id",
    );
    assert_clean_import_rejected(root.path(), &fixture, "file-authority", document, 90)
}

#[test]
fn portable_rejects_agent_authored_host_turn_as_decision_authority(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let mut document = read_json(&fixture.active_bundle)?;
    let sources = table(&document, "sources");
    let id = column(sources, "id");
    let authorization_row = sources["rows"]
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .position(|row| row[id]["value"] == fixture.authorization_id.to_string())
        })
        .ok_or("authorization Source row missing")?;
    set_cell(
        &mut document,
        "sources",
        authorization_row,
        "actor_kind",
        serde_json::json!({"type": "text", "value": "agent"}),
    );
    assert_clean_import_rejected(root.path(), &fixture, "agent-authority", document, 91)
}

#[test]
fn portable_rejects_nonexistent_question_revision() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let mut document = read_json(&fixture.active_bundle)?;
    set_cell(
        &mut document,
        "decisions",
        0,
        "question_revision",
        serde_json::json!({"type": "integer", "value": 99}),
    );
    set_cell(
        &mut document,
        "decision_revisions",
        0,
        "question_revision",
        serde_json::json!({"type": "integer", "value": 99}),
    );
    assert_clean_import_rejected(
        root.path(),
        &fixture,
        "missing-question-revision",
        document,
        92,
    )
}

#[test]
fn portable_rejects_alternative_outside_question_revision() -> Result<(), Box<dyn std::error::Error>>
{
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let mut document = read_json(&fixture.active_bundle)?;
    for table_name in ["decisions", "decision_revisions"] {
        set_cell(
            &mut document,
            table_name,
            0,
            "choice_value",
            serde_json::json!({"type": "text", "value": "forged"}),
        );
    }
    assert_clean_import_rejected(root.path(), &fixture, "invalid-alternative", document, 93)
}

#[test]
fn portable_rejects_question_outcome_and_response_link_mismatch(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let mut outcome = read_json(&fixture.active_bundle)?;
    set_cell(
        &mut outcome,
        "questions",
        0,
        "terminal_outcome",
        serde_json::json!({"type": "text", "value": "delegated"}),
    );
    assert_clean_import_rejected(root.path(), &fixture, "outcome-mismatch", outcome, 94)?;

    let mut missing_link = read_json(&fixture.active_bundle)?;
    table_mut(&mut missing_link, "question_response_sources")["rows"] = Value::Array(Vec::new());
    assert_clean_import_rejected(
        root.path(),
        &fixture,
        "missing-response-link",
        missing_link,
        95,
    )
}

#[test]
fn portable_requires_question_specific_history_despite_unrelated_decision_tombstones(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;

    let mut missing = read_json(&fixture.active_bundle)?;
    table_mut(&mut missing, "question_decision_history_witnesses")["rows"] =
        Value::Array(Vec::new());
    assert_clean_import_rejected(
        root.path(),
        &fixture,
        "missing-history-witness",
        missing,
        121,
    )?;

    for (offset, outcome) in ["answered", "delegated"].into_iter().enumerate() {
        let mut unrelated = read_json(&fixture.active_bundle)?;
        set_cell(
            &mut unrelated,
            "questions",
            0,
            "terminal_outcome",
            serde_json::json!({"type": "text", "value": outcome}),
        );
        for table_name in [
            "question_response_sources",
            "question_decision_history_witnesses",
            "decisions",
            "decision_revisions",
        ] {
            table_mut(&mut unrelated, table_name)["rows"] = Value::Array(Vec::new());
        }
        push_tombstone(&mut unrelated, "decision", portable_id(0x70 + offset as u8));
        assert_clean_import_rejected(
            root.path(),
            &fixture,
            &format!("unrelated-tombstone-{outcome}"),
            unrelated,
            122 + offset as u8,
        )?;
    }
    Ok(())
}

#[test]
fn portable_rejects_mismatched_question_history_witness_fields(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let cases: Vec<(&str, Value)> = vec![
        (
            "wrong-question",
            serde_json::json!({"type": "bytes", "value": "70".repeat(16)}),
        ),
        (
            "wrong-revision",
            serde_json::json!({"type": "integer", "value": 99}),
        ),
        (
            "wrong-outcome",
            serde_json::json!({"type": "text", "value": "delegated"}),
        ),
        (
            "missing-authority",
            serde_json::json!({"type": "text", "value": "unknown"}),
        ),
    ];
    for (offset, (label, value)) in cases.into_iter().enumerate() {
        let mut document = read_json(&fixture.active_bundle)?;
        let column_name = match label {
            "wrong-question" => "question_id",
            "wrong-revision" => "question_revision",
            "wrong-outcome" => "terminal_outcome",
            "missing-authority" => "response_authority",
            _ => return Err("unexpected witness mutation".into()),
        };
        set_cell(
            &mut document,
            "question_decision_history_witnesses",
            0,
            column_name,
            value,
        );
        assert_clean_import_rejected(root.path(), &fixture, label, document, 124 + offset as u8)?;
    }

    let mut wrong_root = read_json(&fixture.active_bundle)?;
    let unrelated_root = portable_id(0x74);
    set_cell(
        &mut wrong_root,
        "question_decision_history_witnesses",
        0,
        "root_decision_id",
        unrelated_root.clone(),
    );
    push_tombstone(&mut wrong_root, "decision", unrelated_root);
    assert_clean_import_rejected(root.path(), &fixture, "wrong-root", wrong_root, 128)?;

    let mut missing_root = read_json(&fixture.active_bundle)?;
    table_mut(&mut missing_root, "decisions")["rows"] = Value::Array(Vec::new());
    table_mut(&mut missing_root, "decision_revisions")["rows"] = Value::Array(Vec::new());
    assert_clean_import_rejected(root.path(), &fixture, "missing-root", missing_root, 135)?;

    let mut unrelated_root_tombstone = read_json(&fixture.active_bundle)?;
    table_mut(&mut unrelated_root_tombstone, "decisions")["rows"] = Value::Array(Vec::new());
    table_mut(&mut unrelated_root_tombstone, "decision_revisions")["rows"] =
        Value::Array(Vec::new());
    push_tombstone(&mut unrelated_root_tombstone, "decision", portable_id(0x75));
    assert_clean_import_rejected(
        root.path(),
        &fixture,
        "unrelated-root-tombstone",
        unrelated_root_tombstone,
        136,
    )?;

    let mut wrong_source = read_json(&fixture.active_bundle)?;
    copy_cell(
        &mut wrong_source,
        "sources",
        0,
        "id",
        "question_decision_history_witnesses",
        0,
        "response_source_id",
    );
    assert_clean_import_rejected(
        root.path(),
        &fixture,
        "wrong-response-source",
        wrong_source,
        129,
    )?;

    let mut duplicate = read_json(&fixture.active_bundle)?;
    let duplicated = table(&duplicate, "question_decision_history_witnesses")["rows"][0].clone();
    table_mut(&mut duplicate, "question_decision_history_witnesses")["rows"]
        .as_array_mut()
        .ok_or("witness rows missing")?
        .push(duplicated);
    assert_clean_import_rejected(root.path(), &fixture, "duplicate-witness", duplicate, 130)?;

    for (operation_byte, outcome) in [
        (131, Value::Null),
        (
            132,
            serde_json::json!({"type": "text", "value": "resolved_by_research"}),
        ),
    ] {
        let mut document = read_json(&fixture.active_bundle)?;
        set_cell(&mut document, "questions", 0, "terminal_outcome", outcome);
        assert_clean_import_rejected(
            root.path(),
            &fixture,
            &format!("witness-on-non-decision-{operation_byte}"),
            document,
            operation_byte,
        )?;
    }
    Ok(())
}

#[test]
fn portable_rejects_decision_revision_provenance_mutation() -> Result<(), Box<dyn std::error::Error>>
{
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let mut document = read_json(&fixture.active_bundle)?;
    copy_cell(
        &mut document,
        "sources",
        0,
        "id",
        "decision_revisions",
        0,
        "user_turn_source_id",
    );
    assert_clean_import_rejected(root.path(), &fixture, "revision-provenance", document, 96)
}

#[test]
fn portable_rejects_mismatched_response_source_and_orphan_decision(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;

    let mut mismatched = read_json(&fixture.active_bundle)?;
    let source = {
        let sources = table(&mismatched, "sources");
        let id = column(sources, "id");
        let row = sources["rows"]
            .as_array()
            .and_then(|rows| {
                rows.iter()
                    .find(|row| row[id]["value"] == fixture.authorization_id.to_string())
            })
            .ok_or("authorization Source missing")?;
        let mut cloned = row.clone();
        cloned[id] = portable_id(0x70);
        cloned
    };
    table_mut(&mut mismatched, "sources")["rows"]
        .as_array_mut()
        .ok_or("Source rows missing")?
        .push(source);
    set_cell(
        &mut mismatched,
        "question_response_sources",
        0,
        "source_id",
        portable_id(0x70),
    );
    assert_clean_import_rejected(root.path(), &fixture, "mismatched-response", mismatched, 97)?;

    let mut orphan = read_json(&fixture.active_bundle)?;
    let mut extra_decision = table(&orphan, "decisions")["rows"][0].clone();
    extra_decision[column(table(&orphan, "decisions"), "id")] = portable_id(0x71);
    table_mut(&mut orphan, "decisions")["rows"]
        .as_array_mut()
        .ok_or("Decision rows missing")?
        .push(extra_decision);
    let mut extra_revision = table(&orphan, "decision_revisions")["rows"][0].clone();
    extra_revision[column(table(&orphan, "decision_revisions"), "decision_id")] = portable_id(0x71);
    table_mut(&mut orphan, "decision_revisions")["rows"]
        .as_array_mut()
        .ok_or("Decision revision rows missing")?
        .push(extra_revision);
    assert_clean_import_rejected(root.path(), &fixture, "orphan-decision", orphan, 98)
}

#[test]
fn portable_rejects_branching_and_cyclic_decision_supersession(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let mut origin = store(&root.path().join("supersession-origin.sqlite3"), &[7])?;
    origin.import_bundle(operation(100), &fixture.active_bundle)?;
    let superseding = origin
        .supersede_decision(
            operation(101),
            fixture.project_id,
            DecisionSupersessionDraft {
                expected_project_revision: 1,
                previous_decision_id: fixture.decision_id,
                user_turn_source: UserTurnSource::Existing(fixture.authorization_id),
                choice: DecisionChoice::Alternative {
                    alternative_key: "central".to_owned(),
                },
                user_rationale: Some("Retain the central boundary".to_owned()),
                applicability: ApplicabilityScope::default(),
                assumptions: vec![],
                revisit_triggers: vec![],
            },
        )?
        .value;
    let valid = root.path().join("valid-supersession.json");
    origin.export_bundle(fixture.project_id, &valid)?;

    let mut branching = read_json(&valid)?;
    let decisions = table(&branching, "decisions");
    let decision_id = column(decisions, "id");
    let superseding_row = decisions["rows"]
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .find(|row| row[decision_id]["value"] == superseding.id.to_string())
        })
        .ok_or("superseding Decision missing")?;
    let mut branch_decision = superseding_row.clone();
    branch_decision[decision_id] = portable_id(0x72);
    table_mut(&mut branching, "decisions")["rows"]
        .as_array_mut()
        .ok_or("Decision rows missing")?
        .push(branch_decision);
    let revisions = table(&branching, "decision_revisions");
    let revision_id = column(revisions, "decision_id");
    let superseding_revision = revisions["rows"]
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .find(|row| row[revision_id]["value"] == superseding.id.to_string())
        })
        .ok_or("superseding Decision revision missing")?;
    let mut branch_revision = superseding_revision.clone();
    branch_revision[revision_id] = portable_id(0x72);
    table_mut(&mut branching, "decision_revisions")["rows"]
        .as_array_mut()
        .ok_or("Decision revision rows missing")?
        .push(branch_revision);
    let relations = table(&branching, "canonical_relations");
    let from_id = column(relations, "from_id");
    let supersession = relations["rows"][0].clone();
    let mut branch_relation = supersession.clone();
    branch_relation[from_id] = portable_id(0x72);
    table_mut(&mut branching, "canonical_relations")["rows"]
        .as_array_mut()
        .ok_or("relation rows missing")?
        .push(branch_relation);
    assert_portable_write_boundaries_reject(
        root.path(),
        &fixture,
        "branching-supersession",
        branching,
    )?;

    let mut cyclic = read_json(&valid)?;
    let relations = table(&cyclic, "canonical_relations");
    let from_id = column(relations, "from_id");
    let to_id = column(relations, "to_id");
    let mut reverse = relations["rows"][0].clone();
    let newer = reverse[from_id].clone();
    reverse[from_id] = reverse[to_id].clone();
    reverse[to_id] = newer;
    table_mut(&mut cyclic, "canonical_relations")["rows"]
        .as_array_mut()
        .ok_or("relation rows missing")?
        .push(reverse);
    assert_portable_write_boundaries_reject(root.path(), &fixture, "cyclic-supersession", cyclic)
}

#[test]
fn portable_rejects_current_snapshot_mismatch_and_semantic_correction(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let mut mismatch = read_json(&fixture.active_bundle)?;
    set_cell(
        &mut mismatch,
        "decisions",
        0,
        "user_rationale",
        serde_json::json!({"type": "text", "value": "mismatched current row"}),
    );
    assert_clean_import_rejected(
        root.path(),
        &fixture,
        "current-snapshot-mismatch",
        mismatch,
        104,
    )?;

    let mut origin = store(&root.path().join("correction-origin.sqlite3"), &[])?;
    origin.import_bundle(operation(105), &fixture.active_bundle)?;
    origin.correct_decision(
        operation(106),
        fixture.project_id,
        fixture.decision_id,
        DecisionCorrectionDraft {
            expected_revision: 1,
            corrected_user_rationale: Some("Keep independently-owned rationale".to_owned()),
            kind: volicord_context::CorrectionKind::Typography,
            user_authorization_source_id: fixture.authorization_id,
        },
    )?;
    let corrected = root.path().join("valid-correction.json");
    origin.export_bundle(fixture.project_id, &corrected)?;
    let mut mutation = read_json(&corrected)?;
    let revisions = table_mut(&mut mutation, "decision_revisions");
    let revision_index = column(revisions, "revision");
    let applicability_index = column(revisions, "applicability_paths");
    let first = revisions["rows"]
        .as_array_mut()
        .and_then(|rows| {
            rows.iter_mut()
                .find(|row| row[revision_index]["value"] == 1)
        })
        .ok_or("initial Decision revision missing")?;
    let encoded = first[applicability_index]["value"]
        .as_str()
        .ok_or("applicability blob missing")?
        .replace("72656275696c642f", "72656275696c782f");
    first[applicability_index]["value"] = Value::String(encoded);
    assert_portable_write_boundaries_reject(root.path(), &fixture, "semantic-correction", mutation)
}

#[test]
fn direct_question_forgetting_bundle_imports_with_sanitized_dependents(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let database = root.path().join("imported.sqlite3");
    let mut imported = store(&database, &[])?;
    imported.import_bundle(operation(30), &fixture.forgotten_bundle)?;
    let decision = imported.get_decision(fixture.project_id, fixture.decision_id)?;
    assert_eq!(
        decision.choice,
        DecisionChoice::Alternative {
            alternative_key: "central".to_owned()
        }
    );
    assert_eq!(
        decision.user_rationale.as_deref(),
        Some("Keep independently owned rationale")
    );
    assert!(decision.displayed_alternatives.is_empty());
    assert!(decision.displayed_recommendation.alternative_key.is_none());
    assert!(decision.displayed_recommendation.rationale.is_empty());
    assert!(decision.displayed_recommendation.source_basis.is_empty());
    assert!(imported
        .get_decision_lifecycle(fixture.project_id, fixture.decision_id)?
        .review_due
        .is_some());
    assert_eq!(
        imported
            .get_question(fixture.project_id, fixture.question_id)
            .err()
            .ok_or("forgotten Question was active")?
            .kind(),
        ErrorKind::NotFound
    );
    Ok(())
}

#[test]
fn import_rejects_recomputed_forgotten_question_presentation_before_mutation(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let active = read_json(&fixture.active_bundle)?;
    let mut invalid = read_json(&fixture.forgotten_bundle)?;
    copy_presentation(&mut invalid, &active, "decisions");
    let path = root.path().join("invalid-active-presentation.json");
    write_recomputed(&path, invalid)?;
    assert_import_rejected_without_mutation(root.path(), &fixture, &path, 40)?;

    let mut checksum_only = fs::read(&fixture.forgotten_bundle)?;
    let position = checksum_only
        .windows(b"Question presentation basis was forgotten".len())
        .position(|window| window == b"Question presentation basis was forgotten")
        .ok_or("review explanation missing")?;
    checksum_only[position] = b'X';
    let checksum_path = root.path().join("checksum-only.json");
    fs::write(&checksum_path, checksum_only)?;
    let mut clean = store(&root.path().join("checksum.sqlite3"), &[])?;
    assert_eq!(
        clean
            .import_bundle(operation(41), &checksum_path)
            .err()
            .ok_or("checksum-corrupt bundle imported")?
            .kind(),
        ErrorKind::IntegrityFailure
    );
    Ok(())
}

#[test]
fn import_rejects_revision_only_presentation_and_missing_review_due(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let active = read_json(&fixture.active_bundle)?;

    let mut revision_only = read_json(&fixture.forgotten_bundle)?;
    copy_presentation(&mut revision_only, &active, "decision_revisions");
    let revision_path = root.path().join("invalid-revision-presentation.json");
    write_recomputed(&revision_path, revision_only)?;
    assert_import_rejected_without_mutation(root.path(), &fixture, &revision_path, 42)?;

    let mut missing_review = read_json(&fixture.forgotten_bundle)?;
    remove_review_due(&mut missing_review);
    let review_path = root.path().join("invalid-missing-review.json");
    write_recomputed(&review_path, missing_review)?;
    assert_import_rejected_without_mutation(root.path(), &fixture, &review_path, 43)?;
    Ok(())
}

#[test]
fn active_question_decision_with_empty_presentation_is_rejected(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let forgotten = read_json(&fixture.forgotten_bundle)?;
    let mut active_empty = read_json(&fixture.active_bundle)?;
    copy_presentation(&mut active_empty, &forgotten, "decisions");
    copy_presentation(&mut active_empty, &forgotten, "decision_revisions");
    assert_clean_import_rejected(root.path(), &fixture, "active-empty", active_empty, 50)
}

#[test]
fn forgotten_question_without_active_decision_does_not_require_review_due(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let database = root.path().join("no-surviving-decision.sqlite3");
    let bundle = root.path().join("no-surviving-decision.json");
    let mut value = store(&database, &[])?;
    value.import_bundle(operation(51), &fixture.active_bundle)?;
    value.forget_decision(
        operation(52),
        fixture.project_id,
        fixture.decision_id,
        fixture.authorization_id,
    )?;
    value.forget_question(
        operation(53),
        fixture.project_id,
        fixture.question_id,
        fixture.authorization_id,
    )?;
    value.export_bundle(fixture.project_id, &bundle)?;
    let mut imported = store(
        &root.path().join("no-surviving-decision-import.sqlite3"),
        &[],
    )?;
    imported.import_bundle(operation(54), &bundle)?;
    assert_eq!(
        imported
            .get_decision(fixture.project_id, fixture.decision_id)
            .err()
            .ok_or("forgotten Decision survived")?
            .kind(),
        ErrorKind::NotFound
    );
    assert_eq!(
        imported
            .get_question(fixture.project_id, fixture.question_id)
            .err()
            .ok_or("forgotten Question survived")?
            .kind(),
        ErrorKind::NotFound
    );
    Ok(())
}

fn assert_explicit_merge_rejected(
    root: &Path,
    fixture: &Fixture,
    invalid: &Path,
    local_question_forgotten: bool,
    local_source_forgotten: bool,
    operation_byte: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let database = root.join(format!("merge-local-{operation_byte}.sqlite3"));
    let managed = root.join(format!("merge-managed-{operation_byte}.json"));
    let clone = root.join(format!("merge-clone-{operation_byte}"));
    fs::create_dir(&clone)?;
    let mut local = store(&database, &[91, 92])?;
    local.import_bundle(operation(60), &fixture.active_bundle)?;
    local.bind_clone(
        operation(62),
        fixture.project_id,
        None,
        &clone,
        Availability::Available,
    )?;
    let forgetting_authority = if local_source_forgotten {
        Some(
            local
                .record_source(operation(63), fixture.project_id, user_turn("merge-forget"))?
                .value,
        )
    } else {
        None
    };
    if local_question_forgotten {
        local.forget_question(
            operation(61),
            fixture.project_id,
            fixture.question_id,
            fixture.authorization_id,
        )?;
    }
    if let Some(authority) = forgetting_authority {
        local.forget_source(
            operation(64),
            fixture.project_id,
            fixture.authorization_id,
            authority.id,
        )?;
    }
    local.export_bundle(fixture.project_id, &managed)?;
    let before_read =
        local.read_canonical_basis(fixture.project_id, CanonicalReadOptions::default())?;
    let before_export = fs::read(&managed)?;
    let before_operations = operation_state(&database)?;
    let before_lineage = lineage_state(&database)?;
    let before_binding = local.get_local_binding(fixture.project_id)?;
    let resolution = MergeResolution {
        conflict_set_identity: "crafted-semantic-boundary".to_owned(),
        conflict_revision: 1,
        user_turn_source_id: fixture.authorization_id,
        mode: MergeResolutionMode::ExplicitMerged {
            bundle_path: invalid.to_path_buf(),
        },
    };
    assert_eq!(
        local
            .merge_bundle(
                operation(operation_byte),
                Some(&fixture.active_bundle),
                &fixture.active_bundle,
                None,
                Some(resolution),
            )
            .err()
            .ok_or("invalid explicit merged state succeeded")?
            .kind(),
        ErrorKind::CorruptState
    );
    assert_eq!(
        local.read_canonical_basis(fixture.project_id, CanonicalReadOptions::default())?,
        before_read
    );
    let repeat = root.join(format!("merge-repeat-{operation_byte}.json"));
    local.export_bundle(fixture.project_id, &repeat)?;
    assert_eq!(fs::read(&repeat)?, before_export);
    assert_eq!(fs::read(&managed)?, before_export);
    assert_eq!(operation_state(&database)?, before_operations);
    assert_eq!(lineage_state(&database)?, before_lineage);
    assert_eq!(local.get_local_binding(fixture.project_id)?, before_binding);
    drop(local);
    let reopened = store(&database, &[])?;
    assert_eq!(
        reopened.read_canonical_basis(fixture.project_id, CanonicalReadOptions::default(),)?,
        before_read
    );
    assert_eq!(
        reopened.get_local_binding(fixture.project_id)?,
        before_binding
    );
    Ok(())
}

#[test]
fn canonical_invariant_mutation_matrix_rejects_every_portable_write_boundary(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let active = read_json(&fixture.active_bundle)?;
    let mut cases = Vec::<(&str, Value)>::new();

    let mut witness_removed = active.clone();
    table_mut(&mut witness_removed, "question_decision_history_witnesses")["rows"] =
        Value::Array(Vec::new());
    cases.push(("witness-removal", witness_removed));

    let mut unrelated_tombstone = active.clone();
    for name in [
        "question_response_sources",
        "question_decision_history_witnesses",
        "decisions",
        "decision_revisions",
    ] {
        table_mut(&mut unrelated_tombstone, name)["rows"] = Value::Array(Vec::new());
    }
    push_tombstone(&mut unrelated_tombstone, "decision", portable_id(0x70));
    cases.push(("unrelated-decision-tombstone", unrelated_tombstone));

    for (label, column_name, value) in [
        ("wrong-witness-question", "question_id", portable_id(0x71)),
        (
            "wrong-witness-revision",
            "question_revision",
            serde_json::json!({"type": "integer", "value": 99}),
        ),
        (
            "wrong-witness-outcome",
            "terminal_outcome",
            serde_json::json!({"type": "text", "value": "delegated"}),
        ),
    ] {
        let mut document = active.clone();
        set_cell(
            &mut document,
            "question_decision_history_witnesses",
            0,
            column_name,
            value,
        );
        cases.push((label, document));
    }

    let mut wrong_root = active.clone();
    set_cell(
        &mut wrong_root,
        "question_decision_history_witnesses",
        0,
        "root_decision_id",
        portable_id(0x72),
    );
    push_tombstone(&mut wrong_root, "decision", portable_id(0x72));
    cases.push(("wrong-root-decision", wrong_root));

    let mut duplicate = active.clone();
    let witness = table(&duplicate, "question_decision_history_witnesses")["rows"][0].clone();
    table_mut(&mut duplicate, "question_decision_history_witnesses")["rows"]
        .as_array_mut()
        .ok_or("witness rows missing")?
        .push(witness);
    cases.push(("duplicate-root-witness", duplicate));

    let mut missing_root = active.clone();
    for name in ["decisions", "decision_revisions"] {
        table_mut(&mut missing_root, name)["rows"] = Value::Array(Vec::new());
    }
    cases.push(("missing-active-root", missing_root));

    let mut forgotten_without_root_tombstone = active.clone();
    for name in ["decisions", "decision_revisions"] {
        table_mut(&mut forgotten_without_root_tombstone, name)["rows"] = Value::Array(Vec::new());
    }
    push_tombstone(
        &mut forgotten_without_root_tombstone,
        "decision",
        portable_id(0x73),
    );
    cases.push((
        "forgotten-root-without-matching-tombstone",
        forgotten_without_root_tombstone,
    ));

    let mut unrelated_response_source = active.clone();
    copy_cell(
        &mut unrelated_response_source,
        "sources",
        0,
        "id",
        "question_decision_history_witnesses",
        0,
        "response_source_id",
    );
    cases.push(("unrelated-response-source", unrelated_response_source));

    let mut non_user_response_authority = active.clone();
    set_cell(
        &mut non_user_response_authority,
        "question_decision_history_witnesses",
        0,
        "response_authority",
        serde_json::json!({"type": "text", "value": "agent_turn"}),
    );
    cases.push(("non-user-response-authority", non_user_response_authority));

    let mut terminal_without_role = active.clone();
    table_mut(
        &mut terminal_without_role,
        "question_decision_history_witnesses",
    )["rows"] = Value::Array(Vec::new());
    cases.push(("terminal-without-role-history", terminal_without_role));

    for (label, outcome) in [
        ("role-on-open-question", Value::Null),
        (
            "role-on-non-decision-question",
            serde_json::json!({"type": "text", "value": "resolved_by_research"}),
        ),
    ] {
        let mut document = active.clone();
        set_cell(&mut document, "questions", 0, "terminal_outcome", outcome);
        cases.push((label, document));
    }

    let mut active_and_tombstoned = active.clone();
    push_tombstone(&mut active_and_tombstoned, "decision", portable_id(5));
    cases.push(("tombstone-plus-active-content", active_and_tombstoned));

    let mut decision_authority = active.clone();
    for name in ["decisions", "decision_revisions"] {
        set_cell(
            &mut decision_authority,
            name,
            0,
            "user_authority",
            serde_json::json!({"type": "text", "value": "agent_turn"}),
        );
    }
    cases.push(("decision-authority", decision_authority));

    let mut question_revision = active.clone();
    for name in ["decisions", "decision_revisions"] {
        set_cell(
            &mut question_revision,
            name,
            0,
            "question_revision",
            serde_json::json!({"type": "integer", "value": 99}),
        );
    }
    cases.push(("decision-question-revision", question_revision));

    let mut decision_alternative = active.clone();
    for name in ["decisions", "decision_revisions"] {
        set_cell(
            &mut decision_alternative,
            name,
            0,
            "choice_value",
            serde_json::json!({"type": "text", "value": "not-displayed"}),
        );
    }
    cases.push(("decision-alternative", decision_alternative));

    let mut local_binding = active;
    local_binding["payload"]["tables"]
        .as_array_mut()
        .ok_or("portable tables missing")?
        .push(serde_json::json!({
            "columns": ["project_id", "clone_path"],
            "name": "local_bindings",
            "rows": [[portable_id(1), {"type": "text", "value": "/local/clone"}]]
        }));
    cases.push(("local-binding-in-portable-state", local_binding));

    for (label, document) in cases {
        assert_portable_write_boundaries_reject(root.path(), &fixture, label, document)?;
    }
    Ok(())
}

#[test]
fn source_context_and_checkpoint_semantic_mutations_reject_every_portable_write_boundary(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = semantic_fixture(root.path())?;
    let active = read_json(&fixture.active_bundle)?;
    let file_row = row_with_text(&active, "sources", "source_kind", "file");
    let command_row = row_with_text(&active, "sources", "source_kind", "command_execution");
    let mut cases = Vec::<(&str, Value)>::new();

    let mut source_kind = active.clone();
    set_cell(
        &mut source_kind,
        "sources",
        command_row,
        "source_kind",
        serde_json::json!({"type": "text", "value": "invented_source"}),
    );
    cases.push(("source-kind", source_kind));

    let mut source_payload = active.clone();
    set_cell(
        &mut source_payload,
        "sources",
        file_row,
        "snapshot_basis",
        Value::Null,
    );
    cases.push(("source-incomplete-payload", source_payload));

    let mut source_combination = active.clone();
    set_cell(
        &mut source_combination,
        "sources",
        command_row,
        "snapshot_basis",
        serde_json::json!({"type": "text", "value": "impossible"}),
    );
    cases.push(("source-impossible-fields", source_combination));

    let mut source_actor = active.clone();
    set_cell(
        &mut source_actor,
        "sources",
        command_row,
        "actor_kind",
        serde_json::json!({"type": "text", "value": "unknown_actor"}),
    );
    cases.push(("source-actor-kind", source_actor));

    let mut source_observer = active.clone();
    set_cell(
        &mut source_observer,
        "sources",
        file_row,
        "observer_kind",
        serde_json::json!({"type": "text", "value": "agent"}),
    );
    cases.push(("source-observer-pair", source_observer));

    let mut source_availability = active.clone();
    set_cell(
        &mut source_availability,
        "sources",
        command_row,
        "availability",
        serde_json::json!({"type": "text", "value": "reachable"}),
    );
    cases.push(("source-availability", source_availability));

    let mut source_absolute = active.clone();
    set_cell(
        &mut source_absolute,
        "sources",
        file_row,
        "locator",
        serde_json::json!({"type": "text", "value": "/tmp/local-only.rs"}),
    );
    cases.push(("source-absolute-locator", source_absolute));

    let mut context_role = active.clone();
    for table_name in ["context_items", "context_item_revisions"] {
        let row_count = table(&context_role, table_name)["rows"]
            .as_array()
            .map(Vec::len)
            .unwrap_or_default();
        for row in 0..row_count {
            set_cell(
                &mut context_role,
                table_name,
                row,
                "provenance_role",
                serde_json::json!({"type": "text", "value": "generated_interpretation"}),
            );
            set_cell(
                &mut context_role,
                table_name,
                row,
                "author_kind",
                serde_json::json!({"type": "text", "value": "generator"}),
            );
        }
    }
    cases.push(("context-fact-reclassification", context_role));

    let mut context_preference = active.clone();
    for table_name in ["context_items", "context_item_revisions"] {
        let row_count = table(&context_preference, table_name)["rows"]
            .as_array()
            .map(Vec::len)
            .unwrap_or_default();
        for row in 0..row_count {
            for (name, value) in [
                ("role", "preference"),
                ("provenance_role", "user_statement"),
                ("author_kind", "user"),
            ] {
                set_cell(
                    &mut context_preference,
                    table_name,
                    row,
                    name,
                    serde_json::json!({"type": "text", "value": value}),
                );
            }
        }
    }
    cases.push(("context-preference-without-user-turn", context_preference));

    let mut context_links = active.clone();
    table_mut(&mut context_links, "context_item_sources")["rows"] = Value::Array(Vec::new());
    cases.push(("context-current-source-links", context_links));

    let mut context_current = active.clone();
    set_cell(
        &mut context_current,
        "context_items",
        0,
        "statement",
        serde_json::json!({"type": "text", "value": "current row diverged"}),
    );
    cases.push(("context-current-revision", context_current));

    let mut context_correction = active.clone();
    set_cell(
        &mut context_correction,
        "context_item_revisions",
        1,
        "statement",
        serde_json::json!({"type": "text", "value": "semantic meaning changed"}),
    );
    set_cell(
        &mut context_correction,
        "context_items",
        0,
        "statement",
        serde_json::json!({"type": "text", "value": "semantic meaning changed"}),
    );
    cases.push(("context-semantic-correction", context_correction));

    let mut context_authority = active.clone();
    set_cell(
        &mut context_authority,
        "context_item_revisions",
        1,
        "authorization_source_id",
        serde_json::json!({"type": "bytes", "value": fixture.file_source_id.to_string()}),
    );
    cases.push(("context-correction-authority", context_authority));

    let mut context_path = active.clone();
    set_cell(
        &mut context_path,
        "context_items",
        0,
        "applicability_paths",
        portable_strings(&["/tmp/local-context"]),
    );
    for row in 0..2 {
        set_cell(
            &mut context_path,
            "context_item_revisions",
            row,
            "applicability_paths",
            portable_strings(&["/tmp/local-context"]),
        );
    }
    cases.push(("context-absolute-applicability", context_path));

    let mut checkpoint_kind = active.clone();
    set_cell(
        &mut checkpoint_kind,
        "checkpoints",
        0,
        "checkpoint_kind",
        serde_json::json!({"type": "text", "value": "pause"}),
    );
    cases.push(("checkpoint-kind-work-state", checkpoint_kind));

    let mut checkpoint_basis = active.clone();
    set_cell(
        &mut checkpoint_basis,
        "checkpoints",
        0,
        "state_change",
        Value::Null,
    );
    set_cell(
        &mut checkpoint_basis,
        "checkpoints",
        0,
        "changed_paths",
        portable_strings(&[]),
    );
    set_cell(
        &mut checkpoint_basis,
        "checkpoints",
        0,
        "known_limits",
        portable_strings(&[]),
    );
    table_mut(&mut checkpoint_basis, "checkpoint_source_relations")["rows"]
        .as_array_mut()
        .ok_or("Checkpoint Source rows missing")?
        .retain(|row| {
            row[column(
                table(&active, "checkpoint_source_relations"),
                "relation_kind",
            )]["value"]
                != "changed_basis"
        });
    for (name, value) in [
        (
            "verification_state",
            serde_json::json!({"type": "text", "value": "not_run"}),
        ),
        ("source_id", Value::Null),
        ("outcome", Value::Null),
    ] {
        set_cell(
            &mut checkpoint_basis,
            "checkpoint_verifications",
            0,
            name,
            value,
        );
    }
    cases.push(("checkpoint-meaningful-basis", checkpoint_basis));

    let mut checkpoint_not_run = active.clone();
    set_cell(
        &mut checkpoint_not_run,
        "checkpoint_verifications",
        0,
        "verification_state",
        serde_json::json!({"type": "text", "value": "not_run"}),
    );
    cases.push((
        "checkpoint-unobserved-verification-claim",
        checkpoint_not_run,
    ));

    let mut checkpoint_verification_source = active.clone();
    set_cell(
        &mut checkpoint_verification_source,
        "checkpoint_verifications",
        0,
        "source_id",
        serde_json::json!({"type": "bytes", "value": fixture.file_source_id.to_string()}),
    );
    cases.push((
        "checkpoint-verification-source-kind",
        checkpoint_verification_source,
    ));

    let mut checkpoint_review = active.clone();
    set_cell(
        &mut checkpoint_review,
        "checkpoints",
        0,
        "user_review_source_id",
        serde_json::json!({"type": "bytes", "value": fixture.file_source_id.to_string()}),
    );
    cases.push(("checkpoint-review-authority", checkpoint_review));

    let mut checkpoint_pending = active.clone();
    set_cell(
        &mut checkpoint_pending,
        "checkpoints",
        0,
        "user_review",
        serde_json::json!({"type": "text", "value": "pending"}),
    );
    cases.push(("checkpoint-unobserved-review-claim", checkpoint_pending));

    let mut checkpoint_acceptance = active.clone();
    set_cell(
        &mut checkpoint_acceptance,
        "checkpoints",
        0,
        "user_acceptance_source_id",
        Value::Null,
    );
    cases.push(("checkpoint-acceptance-source", checkpoint_acceptance));

    let mut checkpoint_path = active.clone();
    set_cell(
        &mut checkpoint_path,
        "checkpoints",
        0,
        "changed_paths",
        portable_strings(&["/tmp/local-checkpoint"]),
    );
    cases.push(("checkpoint-absolute-path", checkpoint_path));

    let mut checkpoint_relation = active.clone();
    set_cell(
        &mut checkpoint_relation,
        "checkpoint_source_relations",
        0,
        "relation_kind",
        serde_json::json!({"type": "text", "value": "claimed_by"}),
    );
    cases.push(("checkpoint-source-relation-kind", checkpoint_relation));

    for (label, document) in cases {
        assert_portable_write_boundaries_reject(root.path(), &fixture, label, document)?;
    }
    Ok(())
}

#[test]
fn explicit_merged_rejects_missing_question_history_before_mutation(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let mut document = read_json(&fixture.active_bundle)?;
    table_mut(&mut document, "question_decision_history_witnesses")["rows"] =
        Value::Array(Vec::new());
    let invalid = root.path().join("missing-history-explicit.json");
    write_recomputed(&invalid, document)?;
    assert_explicit_merge_rejected(root.path(), &fixture, &invalid, false, false, 133)
}

#[test]
fn explicit_merged_rejects_forgotten_question_presentation_before_mutation(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let active = read_json(&fixture.active_bundle)?;
    let mut invalid = read_json(&fixture.forgotten_bundle)?;
    copy_presentation(&mut invalid, &active, "decisions");
    let path = root.path().join("invalid-explicit.json");
    write_recomputed(&path, invalid)?;
    assert_explicit_merge_rejected(root.path(), &fixture, &path, false, false, 70)
}

#[test]
fn explicit_merged_cannot_repopulate_an_already_forgotten_local_question(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let active = read_json(&fixture.active_bundle)?;
    let mut invalid = read_json(&fixture.forgotten_bundle)?;
    copy_presentation(&mut invalid, &active, "decisions");
    let path = root.path().join("invalid-explicit-local-forgotten.json");
    write_recomputed(&path, invalid)?;
    assert_explicit_merge_rejected(root.path(), &fixture, &path, true, false, 71)
}

#[test]
fn explicit_merged_rejects_non_user_decision_with_active_or_forgotten_local_source(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let mut document = read_json(&fixture.active_bundle)?;
    copy_cell(
        &mut document,
        "sources",
        0,
        "id",
        "decisions",
        0,
        "user_turn_source_id",
    );
    copy_cell(
        &mut document,
        "sources",
        0,
        "id",
        "decision_revisions",
        0,
        "user_turn_source_id",
    );
    copy_cell(
        &mut document,
        "sources",
        0,
        "id",
        "question_response_sources",
        0,
        "source_id",
    );
    let path = root.path().join("invalid-non-user-explicit.json");
    write_recomputed(&path, document)?;
    assert_explicit_merge_rejected(root.path(), &fixture, &path, false, false, 72)?;
    assert_explicit_merge_rejected(root.path(), &fixture, &path, false, true, 73)
}

#[test]
fn export_rejects_internal_forgotten_question_state_missing_review_due(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let database = root.path().join("export-invalid.sqlite3");
    let mut value = store(&database, &[])?;
    value.import_bundle(operation(80), &fixture.forgotten_bundle)?;
    let previous = root.path().join("previous.json");
    value.export_bundle(fixture.project_id, &previous)?;
    let previous_bytes = fs::read(&previous)?;
    drop(value);
    Connection::open(&database)?.execute(
        "DELETE FROM review_due WHERE project_id = ?1 AND decision_id = ?2",
        rusqlite::params![
            fixture.project_id.as_bytes().as_slice(),
            fixture.decision_id.as_bytes().as_slice()
        ],
    )?;
    let mut reopened = store(&database, &[])?;
    assert_eq!(
        reopened
            .export_bundle(fixture.project_id, &previous)
            .err()
            .ok_or("inconsistent canonical state was exported")?
            .kind(),
        ErrorKind::CorruptState
    );
    assert_eq!(fs::read(&previous)?, previous_bytes);
    Ok(())
}

#[test]
fn export_rejects_internal_non_user_decision_authority_without_republication(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let database = root.path().join("export-invalid-authority.sqlite3");
    let mut value = store(&database, &[])?;
    value.import_bundle(operation(108), &fixture.active_bundle)?;
    let previous = root.path().join("previous-authority.json");
    value.export_bundle(fixture.project_id, &previous)?;
    let previous_bytes = fs::read(&previous)?;
    drop(value);
    Connection::open(&database)?.execute(
        "UPDATE sources SET actor_kind = 'agent' WHERE project_id = ?1 AND id = ?2",
        rusqlite::params![
            fixture.project_id.as_bytes().as_slice(),
            fixture.authorization_id.as_bytes().as_slice()
        ],
    )?;
    let mut reopened = store(&database, &[])?;
    assert_eq!(
        reopened
            .export_bundle(fixture.project_id, &previous)
            .err()
            .ok_or("internally forged Decision authority was exported")?
            .kind(),
        ErrorKind::CorruptState
    );
    assert_eq!(fs::read(&previous)?, previous_bytes);
    Ok(())
}

#[test]
fn export_rejects_internal_missing_question_history_without_republication(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let database = root.path().join("export-missing-history.sqlite3");
    let mut value = store(&database, &[])?;
    value.import_bundle(operation(134), &fixture.active_bundle)?;
    let previous = root.path().join("previous-history.json");
    value.export_bundle(fixture.project_id, &previous)?;
    let previous_bytes = fs::read(&previous)?;
    drop(value);
    Connection::open(&database)?.execute(
        "DELETE FROM question_decision_history_witnesses WHERE project_id = ?1",
        [fixture.project_id.as_bytes().as_slice()],
    )?;
    let mut reopened = store(&database, &[])?;
    assert_eq!(
        reopened
            .export_bundle(fixture.project_id, &previous)
            .err()
            .ok_or("inconsistent Question history was exported")?
            .kind(),
        ErrorKind::CorruptState
    );
    assert_eq!(fs::read(&previous)?, previous_bytes);
    Ok(())
}

#[test]
fn direct_command_postcondition_rejects_inconsistent_full_state(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let database = root.path().join("direct-postcondition.sqlite3");
    let mut value = store(&database, &[])?;
    value.import_bundle(operation(109), &fixture.active_bundle)?;
    drop(value);
    Connection::open(&database)?.execute(
        "UPDATE sources SET actor_kind = 'agent' WHERE project_id = ?1 AND id = ?2",
        rusqlite::params![
            fixture.project_id.as_bytes().as_slice(),
            fixture.authorization_id.as_bytes().as_slice()
        ],
    )?;

    let mut reopened = store(&database, &[])?;
    assert_eq!(
        reopened
            .rename_project(operation(110), fixture.project_id, 1, "must roll back")
            .err()
            .ok_or("direct command committed an inconsistent Project")?
            .kind(),
        ErrorKind::CorruptState
    );
    assert_eq!(reopened.get_project(fixture.project_id)?.revision, 1);
    assert_eq!(
        operation_state(&database)?
            .iter()
            .filter(|row| row.1 == "rename_project")
            .count(),
        0
    );
    Ok(())
}

#[test]
fn affected_semantic_corruption_blocks_direct_commit_and_production_export(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = semantic_fixture(root.path())?;
    let cases = [
        (
            "source",
            "UPDATE sources SET source_kind = 'invented_source' WHERE source_kind = 'command_execution'",
        ),
        (
            "context",
            "UPDATE context_items SET provenance_role = 'generated_interpretation', author_kind = 'generator';
             UPDATE context_item_revisions SET provenance_role = 'generated_interpretation', author_kind = 'generator'",
        ),
        (
            "checkpoint",
            "UPDATE checkpoints SET work_state = 'paused' WHERE checkpoint_kind = 'completion'",
        ),
    ];
    for (offset, (label, mutation)) in cases.into_iter().enumerate() {
        let database = root.path().join(format!("{label}-internal.sqlite3"));
        let managed = root.path().join(format!("{label}-managed.json"));
        let mut value = store(&database, &[])?;
        value.import_bundle(operation(180 + offset as u8), &fixture.active_bundle)?;
        value.export_bundle(fixture.project_id, &managed)?;
        let before = fs::read(&managed)?;
        drop(value);
        Connection::open(&database)?.execute_batch(mutation)?;

        let mut reopened = store(&database, &[])?;
        assert_eq!(
            reopened
                .export_bundle(fixture.project_id, &managed)
                .err()
                .ok_or("production export admitted corrupt canonical semantics")?
                .kind(),
            ErrorKind::CorruptState
        );
        assert_eq!(fs::read(&managed)?, before);
        assert_eq!(
            reopened
                .rename_project(
                    operation(190 + offset as u8),
                    fixture.project_id,
                    1,
                    format!("blocked-{label}"),
                )
                .err()
                .ok_or("direct transaction committed over corrupt canonical semantics")?
                .kind(),
            ErrorKind::CorruptState
        );
        assert_eq!(reopened.get_project(fixture.project_id)?.revision, 1);
        assert_eq!(
            operation_state(&database)?
                .iter()
                .filter(|row| row.1 == "rename_project")
                .count(),
            0
        );
    }
    Ok(())
}
