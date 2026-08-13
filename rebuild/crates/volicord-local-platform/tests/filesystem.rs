use std::{
    fs,
    process::Command,
    sync::{Arc, Barrier},
    thread,
};

use tempfile::tempdir;
use volicord_local_platform::{
    publish_file_no_replace, DirectoryEntryDurability, DirtyObservation, GitWorktreeLayout,
    NoReplacePublicationOutcome, RepositoryPathState, RepositoryRoot, SourceFingerprint,
};

#[test]
fn paths_normalize_missing_state_and_reject_symlink_escape() {
    use std::os::unix::fs::symlink;
    let temporary = tempdir().expect("temporary directory");
    let root = temporary.path().join("repository");
    let outside = temporary.path().join("outside");
    fs::create_dir_all(root.join("src")).expect("repository");
    fs::create_dir(&outside).expect("outside");
    symlink(&outside, root.join("escape")).expect("symlink");
    let repository = RepositoryRoot::open(&root).expect("root");
    let missing = repository
        .resolve("src/./generated/file.rs")
        .expect("missing path");
    assert_eq!(
        missing.relative(),
        std::path::Path::new("src/generated/file.rs")
    );
    assert_eq!(missing.state(), RepositoryPathState::Missing);
    assert!(repository.resolve("escape/file").is_err());
    assert!(repository.resolve("../outside").is_err());
}

#[test]
fn fingerprints_distinguish_regular_symlink_and_absent_state() {
    use std::os::unix::fs::symlink;
    let temporary = tempdir().expect("temporary directory");
    let regular = temporary.path().join("regular");
    let link = temporary.path().join("link");
    fs::write(&regular, b"bytes").expect("regular");
    symlink("regular", &link).expect("symlink");
    let regular_id = SourceFingerprint::observe(&regular).expect("regular fingerprint");
    let link_id = SourceFingerprint::observe(&link).expect("symlink fingerprint");
    let absent_id =
        SourceFingerprint::observe(temporary.path().join("absent")).expect("absent fingerprint");
    assert_ne!(regular_id, link_id);
    assert_ne!(regular_id, absent_id);
    assert_ne!(link_id, absent_id);
}

#[test]
fn linked_worktrees_share_only_the_local_clone_coordinate() {
    let temporary = tempdir().expect("temporary directory");
    let primary = temporary.path().join("primary");
    let linked = temporary.path().join("linked");
    fs::create_dir(&primary).expect("primary");
    git(&primary, &["init", "-q"]);
    git(&primary, &["config", "user.name", "Fixture"]);
    git(
        &primary,
        &["config", "user.email", "fixture@example.invalid"],
    );
    fs::write(primary.join("file"), b"base").expect("file");
    git(&primary, &["add", "file"]);
    git(&primary, &["commit", "-qm", "fixture"]);
    git(
        &primary,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "linked",
            linked.to_str().expect("path"),
        ],
    );
    let primary_layout = GitWorktreeLayout::resolve(&primary)
        .expect("primary layout")
        .expect("Git");
    let linked_layout = GitWorktreeLayout::resolve(&linked)
        .expect("linked layout")
        .expect("Git");
    assert!(!primary_layout.is_linked_worktree());
    assert!(linked_layout.is_linked_worktree());
    assert_eq!(
        primary_layout.coordinate().clone_identity(),
        linked_layout.coordinate().clone_identity()
    );
    assert_ne!(
        primary_layout.coordinate().worktree_identity(),
        linked_layout.coordinate().worktree_identity()
    );
}

#[test]
fn separate_clone_keeps_a_distinct_local_coordinate() {
    let temporary = tempdir().expect("temporary directory");
    let primary = temporary.path().join("primary");
    let clone = temporary.path().join("clone");
    fs::create_dir(&primary).expect("primary");
    git(&primary, &["init", "-q"]);
    git(&primary, &["config", "user.name", "Fixture"]);
    git(
        &primary,
        &["config", "user.email", "fixture@example.invalid"],
    );
    fs::write(primary.join("file"), b"base").expect("file");
    git(&primary, &["add", "file"]);
    git(&primary, &["commit", "-qm", "fixture"]);
    let output = Command::new("git")
        .args(["clone", "-q"])
        .arg(&primary)
        .arg(&clone)
        .output()
        .expect("git clone");
    assert!(output.status.success());
    let primary_coordinate = GitWorktreeLayout::resolve(&primary)
        .expect("primary layout")
        .expect("Git")
        .coordinate();
    let clone_coordinate = GitWorktreeLayout::resolve(&clone)
        .expect("clone layout")
        .expect("Git")
        .coordinate();
    assert_ne!(
        primary_coordinate.clone_identity(),
        clone_coordinate.clone_identity()
    );
}

#[test]
fn dirty_and_source_fingerprints_change_without_claiming_project_identity() {
    let clean = DirtyObservation::from_porcelain_v2(b"");
    let dirty = DirtyObservation::from_porcelain_v2(b"1 .M N... tracked.txt\0");
    assert!(!clean.is_dirty());
    assert!(dirty.is_dirty());
    assert_ne!(clean.fingerprint(), dirty.fingerprint());
    assert_ne!(
        clean.source_fingerprint(Some("abc")),
        dirty.source_fingerprint(Some("abc"))
    );
}

#[test]
fn publication_is_atomic_no_replace_and_preserves_the_loser() {
    let temporary = tempdir().expect("temporary directory");
    let first = temporary.path().join("first.staging");
    let second = temporary.path().join("second.staging");
    let destination = temporary.path().join("published");
    fs::write(&first, b"first").expect("first");
    fs::write(&second, b"second").expect("second");
    assert_eq!(
        publish_file_no_replace(&first, &destination).expect("publication"),
        NoReplacePublicationOutcome::Published {
            durability: DirectoryEntryDurability::ParentSynchronized
        }
    );
    assert_eq!(
        publish_file_no_replace(&second, &destination).expect("existing"),
        NoReplacePublicationOutcome::DestinationExists
    );
    assert_eq!(fs::read(&destination).expect("published"), b"first");
    assert_eq!(fs::read(&second).expect("loser"), b"second");
}

#[test]
fn concurrent_publishers_choose_exactly_one_complete_file() {
    let temporary = tempdir().expect("temporary directory");
    let destination = temporary.path().join("published");
    let barrier = Arc::new(Barrier::new(3));
    let workers = [
        ("first", b"first".as_slice()),
        ("second", b"second".as_slice()),
    ]
    .map(|(name, bytes)| {
        let source = temporary.path().join(name);
        fs::write(&source, bytes).expect("staged source");
        let destination = destination.clone();
        let barrier = Arc::clone(&barrier);
        thread::spawn(move || {
            barrier.wait();
            (
                source.clone(),
                publish_file_no_replace(&source, &destination),
            )
        })
    });
    barrier.wait();
    let results = workers.map(|worker| worker.join().expect("publisher"));
    assert_eq!(
        results
            .iter()
            .filter(|(_, result)| matches!(
                result,
                Ok(NoReplacePublicationOutcome::Published { .. })
            ))
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|(_, result)| matches!(
                result,
                Ok(NoReplacePublicationOutcome::DestinationExists)
            ))
            .count(),
        1
    );
    assert_eq!(
        results.iter().filter(|(source, _)| source.exists()).count(),
        1
    );
    let bytes = fs::read(destination).expect("published bytes");
    assert!(bytes == b"first" || bytes == b"second");
}

fn git(repository: &std::path::Path, arguments: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(arguments)
        .output()
        .expect("git");
    assert!(
        output.status.success(),
        "git failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
