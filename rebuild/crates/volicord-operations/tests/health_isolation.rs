use std::fs;
use volicord_operations::{HealthIssueKind, HealthState, LocalOperations, RuntimeLayout};

#[cfg(target_os = "linux")]
#[test]
fn unsafe_auxiliary_path_is_not_opened_and_does_not_hide_canonical_health() {
    let temporary = tempfile::tempdir().unwrap();
    let operations =
        LocalOperations::new(RuntimeLayout::new(temporary.path().join("runtime")).unwrap());
    operations.initialize_runtime().unwrap();
    let target = temporary.path().join("outside");
    fs::write(&target, b"do not open this target").unwrap();
    fs::remove_file(operations.layout().privacy_store()).unwrap();
    std::os::unix::fs::symlink(&target, operations.layout().privacy_store()).unwrap();
    let report = operations.health(None);
    assert_eq!(report.state, HealthState::Degraded);
    assert!(report.canonical_available && report.candidate_available);
    assert!(!report.privacy_available);
    assert_eq!(fs::read(target).unwrap(), b"do not open this target");
}

#[test]
fn incompatible_candidates_preserve_usable_memory_and_exact_health_cause() {
    let temporary = tempfile::tempdir().unwrap();
    let operations =
        LocalOperations::new(RuntimeLayout::new(temporary.path().join("runtime")).unwrap());
    let project = operations
        .initialize_project("Memory remains usable", None)
        .unwrap()
        .project
        .id;
    let before = operations.canonical_basis(project).unwrap();
    let connection = rusqlite::Connection::open(operations.layout().candidate_store()).unwrap();
    connection
        .execute(
            "UPDATE metadata SET value = '999' WHERE key = 'schema_version'",
            [],
        )
        .unwrap();
    drop(connection);
    let health = operations.health(Some(project));
    assert_eq!(health.state, HealthState::Degraded);
    assert!(
        health.canonical_available
            && health.privacy_available
            && health.guarded_available
            && health.forgetting_available
    );
    assert!(!health.candidate_available);
    assert!(health.issues.iter().any(|issue| issue.scope == "candidates"
        && issue.kind == HealthIssueKind::Unsupported
        && issue.detail.contains("999")));
    assert_eq!(operations.recall(project).unwrap().project_id, project);
    assert_eq!(before, operations.canonical_basis(project).unwrap());
}

#[test]
fn each_corrupt_store_is_diagnosed_without_masking_other_stores() {
    for damaged in 0..5 {
        let temporary = tempfile::tempdir().unwrap();
        let operations =
            LocalOperations::new(RuntimeLayout::new(temporary.path().join("runtime")).unwrap());
        operations.initialize_runtime().unwrap();
        let paths = [
            operations.layout().canonical_store(),
            operations.layout().candidate_store(),
            operations.layout().privacy_store(),
            operations.layout().guarded_store(),
            operations.layout().forgetting_store(),
        ];
        fs::write(&paths[damaged], b"corrupt store fixture").unwrap();
        let report = operations.health(None);
        let available = [
            report.canonical_available,
            report.candidate_available,
            report.privacy_available,
            report.guarded_available,
            report.forgetting_available,
        ];
        for (index, value) in available.into_iter().enumerate() {
            assert_eq!(value, index != damaged, "damaged={damaged}; {report:?}");
        }
        assert_eq!(
            report.state,
            if damaged == 0 {
                HealthState::Failed
            } else {
                HealthState::Degraded
            }
        );
        assert_eq!(fs::read(&paths[damaged]).unwrap(), b"corrupt store fixture");
    }
}
