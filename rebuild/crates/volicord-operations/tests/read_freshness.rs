use std::fs;
use volicord_operations::{LocalOperations, RuntimeLayout};
use volicord_repository_intelligence::FreshnessState;

#[test]
fn public_reads_compare_source_changes_without_persisting_new_observations() {
    for (file, before_text, after_text) in [
        (
            "main.py",
            "def original():\n    return 1\n",
            "def changed():\n    return 2\n",
        ),
        ("lib.rs", "pub fn original() {}", "pub fn changed() {}"),
        (
            "main.ts",
            "export function original() {}",
            "export function changed() {}",
        ),
    ] {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("repository");
        fs::create_dir(&root).unwrap();
        fs::write(root.join(file), before_text).unwrap();
        fs::write(root.join("excluded.txt"), "not analyzed").unwrap();
        let operations =
            LocalOperations::new(RuntimeLayout::new(temporary.path().join("runtime")).unwrap());
        let project = operations
            .initialize_project("Freshness fixture", Some(&root))
            .unwrap()
            .project
            .id;
        let analysis = operations
            .analyze(project, vec!["excluded.txt".into()])
            .unwrap()
            .value
            .unwrap();
        let canonical = operations.canonical_basis(project).unwrap();
        let cached = fs::read(&analysis.stored_at).unwrap();
        assert_eq!(
            operations.recall(project).unwrap().snapshots[0]
                .freshness
                .state,
            FreshnessState::Current
        );
        // Excluded content is not read; keep observed size metadata unchanged.
        fs::write(root.join("excluded.txt"), "also ignored").unwrap();
        assert_eq!(
            operations.recall(project).unwrap().snapshots[0]
                .freshness
                .state,
            FreshnessState::Current
        );
        fs::write(root.join(file), after_text).unwrap();
        let brief = operations.recall(project).unwrap();
        assert_eq!(brief.snapshots[0].freshness.state, FreshnessState::Stale);
        assert_eq!(
            brief.snapshots[0].analysis_snapshot,
            analysis.analysis.identity
        );
        assert_ne!(
            brief.snapshots[0].freshness.compared_repository_snapshot,
            Some(analysis.repository.identity)
        );
        let projection = operations.project_projection(project).unwrap();
        assert!(!projection.repository_map.entities.is_empty());
        assert!(projection
            .repository_map
            .entities
            .iter()
            .all(|entity| entity.freshness.state == FreshnessState::Stale));
        assert!(projection
            .repository_map
            .relations
            .iter()
            .all(|relation| relation.freshness.state == FreshnessState::Stale));
        assert!(projection
            .issues
            .iter()
            .any(|issue| issue.reason.contains("repository has changed")));
        fs::remove_dir_all(&root).unwrap();
        let unavailable = operations.project_projection(project).unwrap();
        assert!(unavailable
            .repository_map
            .entities
            .iter()
            .all(|entity| entity.freshness.state == FreshnessState::Unknown));
        assert_eq!(canonical, operations.canonical_basis(project).unwrap());
        assert_eq!(cached, fs::read(&analysis.stored_at).unwrap());
        assert_eq!(
            fs::read_dir(operations.layout().analysis_project_dir(project))
                .unwrap()
                .count(),
            1
        );
    }
}
