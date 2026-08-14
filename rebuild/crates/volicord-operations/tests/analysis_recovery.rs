use std::{fs, path::PathBuf};
use tempfile::TempDir;
use volicord_context::{
    ApplicabilityScope, Availability, ContextItemCorrectionDraft, ContextItemDraft,
    ContextItemRole, CorrectionKind, OperationId, Principal, PrincipalKind, SourceDraft,
    SourcePayload, StatementProvenanceRole, Store,
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

fn add_corrected_context(
    operations: &LocalOperations,
    project: volicord_context::ProjectId,
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
    drop(store);
    operations.correct_context_item(
        project,
        item.id,
        ContextItemCorrectionDraft {
            expected_revision: item.revision,
            corrected_statement: "keep derived state local".into(),
            kind: CorrectionKind::Typography,
            user_authorization_source_id: user.id,
        },
    )?;
    Ok(())
}

#[test]
fn corrupt_analysis_is_diagnosed_repaired_and_canonical_bytes_are_unchanged(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let initialized = fixture
        .operations
        .initialize_project("Repair fixture", Some(&fixture.repository))?;
    let analyzed = fixture
        .operations
        .analyze(initialized.project.id, Vec::new())?;
    let stored = analyzed.value.ok_or("analysis result missing")?.stored_at;
    add_corrected_context(&fixture.operations, initialized.project.id)?;
    let before_basis = fixture.operations.canonical_basis(initialized.project.id)?;
    let before_bundle = export_bytes(
        &fixture.operations,
        initialized.project.id,
        &fixture._temporary.path().join("before.json"),
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
    let after_bundle = export_bytes(
        &fixture.operations,
        initialized.project.id,
        &fixture._temporary.path().join("after.json"),
    )?;
    assert_eq!(
        before_basis.stable_ordering_identity,
        after_basis.stable_ordering_identity
    );
    assert_eq!(before_bundle, after_bundle);
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
    assert_eq!(before_bundle, after_bundle);
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
