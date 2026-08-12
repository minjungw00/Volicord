use rusqlite::Connection;
use std::fs;
use std::path::Path;
use tempfile::tempdir;
use volicord_context::{
    Availability, CommandOutcome, CommandTermination, DeterministicIdGenerator, ErrorKind,
    FixedClock, OperationId, Principal, PrincipalKind, ProjectId, SourceDraft, SourceId,
    SourcePayload, SourceRelationKind, Store, TimestampMicros, SCHEMA_KIND, SCHEMA_VERSION,
};

fn operation(value: u8) -> OperationId {
    OperationId::from_bytes([value; 16])
}

fn store_with_ids(path: &Path, values: &[u8]) -> Result<Store, volicord_context::Error> {
    Store::open_with(
        path,
        DeterministicIdGenerator::new(values.iter().map(|value| [*value; 16])),
        FixedClock::new(TimestampMicros::from_unix_micros(1_735_689_600_123_456)),
    )
}

fn actor(kind: PrincipalKind, identity: &str) -> Principal {
    Principal {
        kind,
        identity: identity.to_owned(),
    }
}

fn source_draft(payload: SourcePayload, actor_kind: PrincipalKind, actor_id: &str) -> SourceDraft {
    SourceDraft {
        expected_project_revision: 1,
        payload,
        actor: actor(actor_kind, actor_id),
        observer: Some(actor(PrincipalKind::Agent, "observer-agent")),
        availability: Availability::Available,
    }
}

#[test]
fn creates_schema_with_required_durability_profile() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let path = root.path().join("context.sqlite3");
    let store = store_with_ids(&path, &[])?;
    assert_eq!(store.path(), path);
    drop(store);

    let connection = Connection::open(&path)?;
    let kind: String = connection.query_row(
        "SELECT value FROM metadata WHERE key = 'schema_kind'",
        [],
        |row| row.get(0),
    )?;
    let version: String = connection.query_row(
        "SELECT value FROM metadata WHERE key = 'schema_version'",
        [],
        |row| row.get(0),
    )?;
    let journal: String = connection.query_row("PRAGMA journal_mode", [], |row| row.get(0))?;
    let synchronous: i64 = connection.query_row("PRAGMA synchronous", [], |row| row.get(0))?;
    assert_eq!(kind, SCHEMA_KIND);
    assert_eq!(version, SCHEMA_VERSION.to_string());
    assert!(journal.eq_ignore_ascii_case("wal"));
    assert_eq!(synchronous, 2);
    let operation_columns: Vec<String> = connection
        .prepare("PRAGMA table_info(operations)")?
        .query_map([], |row| row.get(1))?
        .collect::<Result<_, _>>()?;
    assert_eq!(
        operation_columns,
        vec![
            "operation_id",
            "project_id",
            "operation_kind",
            "input_basis",
            "replay_state",
            "outcome",
            "result_kind",
            "result_id",
            "result_revision",
            "committed_at",
        ]
    );
    let dependency_columns: Vec<String> = connection
        .prepare("PRAGMA table_info(operation_dependencies)")?
        .query_map([], |row| row.get(1))?
        .collect::<Result<_, _>>()?;
    assert_eq!(
        dependency_columns,
        vec!["operation_id", "project_id", "owner_kind", "owner_id"]
    );
    Ok(())
}

#[test]
fn project_identity_survives_rename_rebind_and_database_location_change(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let path = root.path().join("primary.sqlite3");
    let first_clone = root.path().join("clone-a");
    let second_clone = root.path().join("clone-b");
    fs::create_dir(&first_clone)?;
    fs::create_dir(&second_clone)?;

    let mut store = store_with_ids(&path, &[1, 2, 3])?;
    let created = store.create_project(operation(10), "Original")?.value;
    assert_eq!(created.id, ProjectId::from_bytes([1; 16]));
    let first_binding = store
        .bind_clone(
            operation(11),
            created.id,
            None,
            &first_clone,
            Availability::Available,
        )?
        .value;
    let historical_source = store
        .record_source(
            operation(12),
            created.id,
            source_draft(
                SourcePayload::File {
                    locator: "src/lib.rs".to_owned(),
                    snapshot: "commit-a".to_owned(),
                },
                PrincipalKind::Repository,
                "repository-observer",
            ),
        )?
        .value;
    let renamed = store
        .rename_project(operation(13), created.id, 1, "Renamed")?
        .value;
    let rebound = store
        .bind_clone(
            operation(14),
            created.id,
            Some(first_binding.revision),
            &second_clone,
            Availability::Available,
        )?
        .value;

    assert_eq!(renamed.id, created.id);
    assert_eq!(rebound.project_id, created.id);
    assert_eq!(rebound.id, first_binding.id);
    assert_eq!(rebound.revision, 2);
    assert_eq!(
        store.get_source(created.id, historical_source.id)?.payload,
        SourcePayload::File {
            locator: "src/lib.rs".to_owned(),
            snapshot: "commit-a".to_owned(),
        }
    );
    drop(store);

    let moved_path = root.path().join("moved.sqlite3");
    fs::copy(&path, &moved_path)?;
    let moved = store_with_ids(&moved_path, &[])?;
    assert_eq!(moved.get_project(created.id)?.id, created.id);
    assert_eq!(moved.get_project(created.id)?.display_name, "Renamed");
    assert_eq!(
        moved.get_local_binding(created.id)?.absolute_path,
        second_clone
    );
    Ok(())
}

#[test]
fn records_all_required_source_kinds_and_provenance() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let mut store = store_with_ids(
        &root.path().join("context.sqlite3"),
        &[1, 2, 3, 4, 5, 6, 7, 8, 9],
    )?;
    let project = store.create_project(operation(20), "Sources")?.value;
    let payloads = vec![
        SourcePayload::RepositorySnapshot {
            revision: "tree-1".to_owned(),
        },
        SourcePayload::RepositoryCommit {
            commit: "abc123".to_owned(),
        },
        SourcePayload::File {
            locator: "src/main.rs".to_owned(),
            snapshot: "abc123".to_owned(),
        },
        SourcePayload::Symbol {
            locator: "src/main.rs::main".to_owned(),
            snapshot: "abc123".to_owned(),
        },
        SourcePayload::CommandExecution {
            command_label: "cargo test (bounded observation)".to_owned(),
            outcome: CommandOutcome {
                exit_code: Some(0),
                termination: CommandTermination::Exited,
            },
        },
        SourcePayload::CurrentHostUserTurn {
            host: "codex".to_owned(),
            session: "session-1".to_owned(),
            turn: "turn-3".to_owned(),
        },
        SourcePayload::Url {
            url: "https://example.invalid/reference".to_owned(),
        },
        SourcePayload::AdoptedArtifact {
            locator: "docs/design.md".to_owned(),
            revision: "artifact-7".to_owned(),
        },
    ];

    for (index, payload) in payloads.into_iter().enumerate() {
        let source = store
            .record_source(
                operation(30 + index as u8),
                project.id,
                source_draft(payload.clone(), PrincipalKind::User, "current-user"),
            )?
            .value;
        let read = store.get_source(project.id, source.id)?;
        assert_eq!(read.payload, payload);
        assert_eq!(read.actor, actor(PrincipalKind::User, "current-user"));
        assert_eq!(
            read.observer,
            Some(actor(PrincipalKind::Agent, "observer-agent"))
        );
    }
    Ok(())
}

#[test]
fn rejects_cross_project_relations_and_bindings() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let path = root.path().join("context.sqlite3");
    let clone = root.path().join("clone");
    fs::create_dir(&clone)?;
    let mut store = store_with_ids(&path, &[1, 2, 3, 4, 5])?;
    let first = store.create_project(operation(40), "First")?.value;
    let second = store.create_project(operation(41), "Second")?.value;
    store.bind_clone(
        operation(42),
        first.id,
        None,
        &clone,
        Availability::Available,
    )?;
    let binding_error = store
        .bind_clone(
            operation(43),
            second.id,
            None,
            &clone,
            Availability::Available,
        )
        .err()
        .ok_or("expected a wrong-Project binding error")?;
    assert_eq!(binding_error.kind(), ErrorKind::WrongProject);

    let first_source = store
        .record_source(
            operation(44),
            first.id,
            source_draft(
                SourcePayload::Url {
                    url: "https://example.invalid/one".to_owned(),
                },
                PrincipalKind::Agent,
                "agent",
            ),
        )?
        .value;
    let second_source = store
        .record_source(
            operation(45),
            second.id,
            source_draft(
                SourcePayload::Url {
                    url: "https://example.invalid/two".to_owned(),
                },
                PrincipalKind::Agent,
                "agent",
            ),
        )?
        .value;
    let relation_error = store
        .relate_sources(
            operation(46),
            first.id,
            1,
            first_source.id,
            SourceRelationKind::DerivedFrom,
            second_source.id,
        )
        .err()
        .ok_or("expected a wrong-Project relation error")?;
    assert_eq!(relation_error.kind(), ErrorKind::WrongProject);
    assert_eq!(
        store
            .get_source_relation(
                first.id,
                first_source.id,
                SourceRelationKind::DerivedFrom,
                second_source.id,
            )
            .err()
            .ok_or("expected no cross-Project relation")?
            .kind(),
        ErrorKind::NotFound
    );
    Ok(())
}

#[test]
fn operation_replay_preserves_prior_result_and_detects_mismatch_and_stale_basis(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let path = root.path().join("context.sqlite3");
    let mut store = store_with_ids(&path, &[1])?;
    let created = store.create_project(operation(50), "Replay")?.value;
    store.rename_project(operation(51), created.id, 1, "Current")?;

    let replay = store.create_project(operation(50), "Replay")?;
    assert!(replay.replayed);
    assert_eq!(replay.value.display_name, "Replay");
    assert_eq!(replay.value.revision, 1);
    assert_eq!(store.get_project(created.id)?.display_name, "Current");

    let mismatch = store
        .create_project(operation(50), "Different input")
        .err()
        .ok_or("expected replay mismatch")?;
    assert_eq!(mismatch.kind(), ErrorKind::DomainConflict);
    let stale = store
        .rename_project(operation(52), created.id, 1, "Stale")
        .err()
        .ok_or("expected stale basis")?;
    assert_eq!(stale.kind(), ErrorKind::StaleBasis);
    assert_eq!(store.get_project(created.id)?.display_name, "Current");
    drop(store);

    let mut reopened = store_with_ids(&path, &[])?;
    let recovered = reopened.create_project(operation(50), "Replay")?;
    assert!(recovered.replayed);
    assert_eq!(recovered.value.id, created.id);
    Ok(())
}

#[test]
fn failed_operation_rolls_back_relation_and_operation_record(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let path = root.path().join("context.sqlite3");
    let mut store = store_with_ids(&path, &[1, 2, 3])?;
    let project = store.create_project(operation(60), "Rollback")?.value;
    let one = store
        .record_source(
            operation(61),
            project.id,
            source_draft(
                SourcePayload::Url {
                    url: "https://example.invalid/one".to_owned(),
                },
                PrincipalKind::Agent,
                "agent",
            ),
        )?
        .value;
    let two = store
        .record_source(
            operation(62),
            project.id,
            source_draft(
                SourcePayload::Url {
                    url: "https://example.invalid/two".to_owned(),
                },
                PrincipalKind::Agent,
                "agent",
            ),
        )?
        .value;
    drop(store);

    let fault = Connection::open(&path)?;
    fault.execute_batch(
        "CREATE TRIGGER fail_relation_operation
         BEFORE INSERT ON operations
         WHEN NEW.operation_kind = 'relate_sources'
         BEGIN SELECT RAISE(ABORT, 'injected operation-log failure'); END;",
    )?;
    drop(fault);

    let mut store = store_with_ids(&path, &[])?;
    let error = store
        .relate_sources(
            operation(63),
            project.id,
            1,
            one.id,
            SourceRelationKind::SupportedBy,
            two.id,
        )
        .err()
        .ok_or("expected injected transaction failure")?;
    assert_eq!(error.kind(), ErrorKind::TransactionFailure);
    assert_eq!(
        store
            .get_source_relation(project.id, one.id, SourceRelationKind::SupportedBy, two.id,)
            .err()
            .ok_or("expected relation rollback")?
            .kind(),
        ErrorKind::NotFound
    );
    drop(store);

    let fault = Connection::open(&path)?;
    fault.execute_batch("DROP TRIGGER fail_relation_operation")?;
    drop(fault);
    let mut store = store_with_ids(&path, &[])?;
    let result = store.relate_sources(
        operation(63),
        project.id,
        1,
        one.id,
        SourceRelationKind::SupportedBy,
        two.id,
    )?;
    assert!(!result.replayed);
    Ok(())
}

#[test]
fn close_and_reopen_preserves_identity_binding_and_provenance(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let path = root.path().join("context.sqlite3");
    let clone = root.path().join("clone");
    fs::create_dir(&clone)?;
    let mut store = store_with_ids(&path, &[1, 2, 3])?;
    let project = store.create_project(operation(70), "Reopen")?.value;
    let binding = store
        .bind_clone(
            operation(71),
            project.id,
            None,
            &clone,
            Availability::Available,
        )?
        .value;
    let source = store
        .record_source(
            operation(72),
            project.id,
            source_draft(
                SourcePayload::CurrentHostUserTurn {
                    host: "codex".to_owned(),
                    session: "session".to_owned(),
                    turn: "turn".to_owned(),
                },
                PrincipalKind::User,
                "user",
            ),
        )?
        .value;
    drop(store);

    let reopened = store_with_ids(&path, &[])?;
    assert_eq!(reopened.get_project(project.id)?, project);
    assert_eq!(reopened.get_local_binding(project.id)?, binding);
    assert_eq!(reopened.get_source(project.id, source.id)?, source);
    Ok(())
}

#[test]
fn rejects_unsupported_newer_schema_without_mutating_file() -> Result<(), Box<dyn std::error::Error>>
{
    let root = tempdir()?;
    let path = root.path().join("context.sqlite3");
    drop(store_with_ids(&path, &[])?);
    let connection = Connection::open(&path)?;
    connection.execute(
        "UPDATE metadata SET value = ?1 WHERE key = 'schema_version'",
        [(SCHEMA_VERSION + 1).to_string()],
    )?;
    drop(connection);
    let before = fs::read(&path)?;

    let error = Store::open(&path)
        .err()
        .ok_or("expected unsupported schema rejection")?;
    assert_eq!(error.kind(), ErrorKind::UnsupportedVersion);
    assert_eq!(fs::read(&path)?, before);
    Ok(())
}

#[test]
fn rejects_corrupt_empty_and_malformed_stores_without_fresh_initialization(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;

    let corrupt = root.path().join("corrupt.sqlite3");
    fs::write(&corrupt, b"not a sqlite database")?;
    let error = Store::open(&corrupt)
        .err()
        .ok_or("expected corrupt store rejection")?;
    assert_eq!(error.kind(), ErrorKind::CorruptState);
    assert_eq!(fs::read(&corrupt)?, b"not a sqlite database");

    let empty = root.path().join("empty.sqlite3");
    fs::write(&empty, [])?;
    let error = Store::open(&empty)
        .err()
        .ok_or("expected empty existing store rejection")?;
    assert_eq!(error.kind(), ErrorKind::CorruptState);

    let malformed = root.path().join("malformed.sqlite3");
    drop(store_with_ids(&malformed, &[])?);
    let connection = Connection::open(&malformed)?;
    connection.execute(
        "UPDATE metadata SET value = 'not-a-number' WHERE key = 'schema_version'",
        [],
    )?;
    drop(connection);
    let error = Store::open(&malformed)
        .err()
        .ok_or("expected malformed metadata rejection")?;
    assert_eq!(error.kind(), ErrorKind::CorruptState);
    Ok(())
}

#[test]
fn generated_ids_are_injected_and_not_derived_from_content(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let first_path = root.path().join("first.sqlite3");
    let second_path = root.path().join("second.sqlite3");
    let mut first = store_with_ids(&first_path, &[91])?;
    let mut second = store_with_ids(&second_path, &[92])?;
    let first_project = first.create_project(operation(80), "Same name")?.value;
    let second_project = second.create_project(operation(80), "Same name")?.value;
    assert_eq!(first_project.id, ProjectId::from_bytes([91; 16]));
    assert_eq!(second_project.id, ProjectId::from_bytes([92; 16]));
    assert_ne!(first_project.id, second_project.id);
    Ok(())
}

#[test]
fn typed_lookup_cannot_cross_source_identity_and_project_scope(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let path = root.path().join("context.sqlite3");
    let mut store = store_with_ids(&path, &[1, 2])?;
    let project = store.create_project(operation(90), "Typed")?.value;
    let source = store
        .record_source(
            operation(91),
            project.id,
            source_draft(
                SourcePayload::RepositoryCommit {
                    commit: "deadbeef".to_owned(),
                },
                PrincipalKind::Repository,
                "git",
            ),
        )?
        .value;
    assert_eq!(source.id, SourceId::from_bytes([2; 16]));
    let wrong_project = store
        .get_source(ProjectId::from_bytes([99; 16]), source.id)
        .err()
        .ok_or("expected wrong-Project lookup")?;
    assert_eq!(wrong_project.kind(), ErrorKind::WrongProject);
    Ok(())
}
