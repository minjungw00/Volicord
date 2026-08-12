use rusqlite::Connection;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use volicord_context::{
    AgentRecommendation, ApplicabilityScope, Availability, CanonicalReadOptions, DecisionChoice,
    DeterministicIdGenerator, ErrorKind, ExplicitQuestionResponse, FixedClock, MergeResolution,
    MergeResolutionMode, OperationId, Principal, PrincipalKind, ProjectId, QuestionAlternative,
    QuestionDraft, QuestionId, QuestionResponseDraft, SourceDraft, SourceId, SourcePayload, Store,
    TimestampMicros, UserTurnSource,
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
        authorization_id: authorization.id,
        active_bundle,
        forgotten_bundle,
    })
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

fn lineage_state(path: &Path) -> Result<Vec<(Vec<u8>, String)>, Box<dyn std::error::Error>> {
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
fn active_question_decision_with_empty_presentation_is_valid(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let forgotten = read_json(&fixture.forgotten_bundle)?;
    let mut active_empty = read_json(&fixture.active_bundle)?;
    copy_presentation(&mut active_empty, &forgotten, "decisions");
    copy_presentation(&mut active_empty, &forgotten, "decision_revisions");
    let path = root.path().join("active-empty.json");
    write_recomputed(&path, active_empty)?;
    let mut imported = store(&root.path().join("active-empty.sqlite3"), &[])?;
    imported.import_bundle(operation(50), &path)?;
    let decision = imported.get_decision(fixture.project_id, fixture.decision_id)?;
    assert!(decision.displayed_alternatives.is_empty());
    assert!(imported
        .get_decision_lifecycle(fixture.project_id, fixture.decision_id)?
        .review_due
        .is_none());
    Ok(())
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
    operation_byte: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let database = root.join(format!("merge-local-{operation_byte}.sqlite3"));
    let managed = root.join(format!("merge-managed-{operation_byte}.json"));
    let clone = root.join(format!("merge-clone-{operation_byte}"));
    fs::create_dir(&clone)?;
    let mut local = store(&database, &[91])?;
    local.import_bundle(operation(60), &fixture.active_bundle)?;
    local.bind_clone(
        operation(62),
        fixture.project_id,
        None,
        &clone,
        Availability::Available,
    )?;
    if local_question_forgotten {
        local.forget_question(
            operation(61),
            fixture.project_id,
            fixture.question_id,
            fixture.authorization_id,
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
fn explicit_merged_rejects_forgotten_question_presentation_before_mutation(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let fixture = fixture(root.path())?;
    let active = read_json(&fixture.active_bundle)?;
    let mut invalid = read_json(&fixture.forgotten_bundle)?;
    copy_presentation(&mut invalid, &active, "decisions");
    let path = root.path().join("invalid-explicit.json");
    write_recomputed(&path, invalid)?;
    assert_explicit_merge_rejected(root.path(), &fixture, &path, false, 70)
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
    assert_explicit_merge_rejected(root.path(), &fixture, &path, true, 71)
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
