use std::{
    fs,
    process::Command,
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};

use tempfile::tempdir;
use volicord_local_platform::{
    ensure_private_directory, ensure_private_file, publish_file_no_replace,
    DirectoryEntryDurability, DirtyObservation, GitWorktreeLayout, MutationLockGuard,
    NoReplacePublicationEffect, NoReplacePublicationOutcome, NoReplacePublicationPhase,
    RepositoryPathState, RepositoryRoot, SourceFingerprint,
};

#[cfg(target_os = "linux")]
use std::os::unix::fs::{symlink, PermissionsExt};

#[cfg(target_os = "linux")]
struct KillAndReap(std::process::Child);

#[cfg(target_os = "linux")]
impl Drop for KillAndReap {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

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
fn repository_name_hint_uses_unambiguous_local_origin_metadata() {
    let temporary = tempdir().expect("temporary directory");
    let repository = temporary.path().join("repository");
    fs::create_dir(&repository).expect("repository");
    git(&repository, &["init", "-q"]);

    for (origin, expected) in [
        (
            "https://github.com/tree-sitter/tree-sitter.git",
            Some("tree-sitter"),
        ),
        (
            "git@github.com:pallets/itsdangerous.git",
            Some("itsdangerous"),
        ),
        ("/var/tmp/Volicord.git", Some("Volicord")),
        (
            "file:///var/tmp/source-repository.git",
            Some("source-repository"),
        ),
        ("/", None),
        ("https://example.invalid/unsafe%20name.git", None),
    ] {
        git(&repository, &["remote", "add", "origin", origin]);
        let layout = GitWorktreeLayout::resolve(&repository)
            .expect("layout")
            .expect("Git repository");
        assert_eq!(
            layout.repository_name_hint().as_deref(),
            expected,
            "{origin}"
        );
        git(&repository, &["remote", "remove", "origin"]);
    }
}

#[test]
fn filesystem_origin_survives_cloning_into_a_generic_directory_name() {
    let temporary = tempdir().expect("temporary directory");
    let source = temporary.path().join("tree-sitter");
    let clone = temporary.path().join("campaign").join("repository");
    fs::create_dir_all(&source).expect("source");
    git(&source, &["init", "-q"]);
    fs::create_dir_all(clone.parent().expect("clone parent")).expect("campaign");
    let output = Command::new("git")
        .args(["clone", "-q"])
        .arg(&source)
        .arg(&clone)
        .output()
        .expect("git clone");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let layout = GitWorktreeLayout::resolve(&clone)
        .expect("layout")
        .expect("Git clone");
    assert_eq!(
        layout.repository_name_hint().as_deref(),
        Some("tree-sitter")
    );
}

#[test]
fn dirty_and_source_fingerprints_change_without_claiming_project_identity() {
    let clean = DirtyObservation::from_porcelain_v2(b"").expect("clean status");
    let dirty = DirtyObservation::from_porcelain_v2(
        b"1 .M N... 100644 100644 100644 abcdef abcdef tracked.txt\0\
2 R. N... 100644 100644 100644 abcdef abcdef R100 renamed.txt\0original.txt\0\
2 C. N... 100644 100644 100644 abcdef abcdef C100 copied.txt\0source.txt\0\
1 D. N... 100644 000000 000000 abcdef 000000 deleted.txt\0\
? untracked/nested.txt\0",
    )
    .expect("dirty status");
    assert!(!clean.is_dirty());
    assert!(dirty.is_dirty());
    assert_eq!(
        dirty.dirty_paths(),
        [
            "copied.txt",
            "deleted.txt",
            "original.txt",
            "renamed.txt",
            "tracked.txt",
            "untracked/nested.txt",
        ]
    );
    assert_ne!(clean.fingerprint(), dirty.fingerprint());
    assert_ne!(
        clean.source_fingerprint(Some("abc")),
        dirty.source_fingerprint(Some("abc"))
    );
}

#[test]
fn dirty_observation_rejects_non_portable_or_malformed_paths() {
    for value in [
        b"? ../outside.txt\0".as_slice(),
        b"? /absolute.txt\0".as_slice(),
        b"2 R. N... 100644 100644 100644 abcdef abcdef R100 renamed.txt\0".as_slice(),
        b"unexpected\0".as_slice(),
    ] {
        assert!(DirtyObservation::from_porcelain_v2(value).is_err());
    }
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

#[cfg(target_os = "linux")]
#[test]
fn private_runtime_paths_repair_owned_modes_and_reject_symlinks() {
    let temporary = tempdir().expect("temporary directory");
    let directory = temporary.path().join("runtime");
    let file = directory.join("state.sqlite3");
    fs::create_dir(&directory).expect("runtime");
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o777)).expect("directory mode");
    fs::write(&file, b"state").expect("state");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o666)).expect("file mode");

    ensure_private_directory(&directory).expect("private directory");
    ensure_private_file(&file).expect("private file");
    assert_eq!(
        fs::symlink_metadata(&directory)
            .unwrap()
            .permissions()
            .mode()
            & 0o7777,
        0o700
    );
    assert_eq!(
        fs::symlink_metadata(&file).unwrap().permissions().mode() & 0o7777,
        0o600
    );

    let linked_file = directory.join("linked");
    symlink(&file, &linked_file).expect("symlink");
    assert!(ensure_private_file(&linked_file).is_err());
}

#[cfg(target_os = "linux")]
#[test]
fn private_runtime_creation_probe() {
    let Some(root) = std::env::var_os("VOLICORD_TEST_PRIVATE_RUNTIME") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    ensure_private_directory(&root).expect("private runtime");
    ensure_private_file(&root.join("state")).expect("private state");
}

#[cfg(target_os = "linux")]
#[test]
fn private_runtime_creation_is_independent_of_permissive_umask() {
    let temporary = tempdir().expect("temporary directory");
    let runtime = temporary.path().join("nested/runtime");
    let executable = std::env::current_exe().expect("current test executable");
    let status = Command::new("sh")
        .args([
            "-c",
            "umask 000; exec \"$1\" --exact private_runtime_creation_probe --nocapture",
            "private-runtime-probe",
        ])
        .arg(executable)
        .env("VOLICORD_TEST_PRIVATE_RUNTIME", &runtime)
        .status()
        .expect("private runtime probe");
    assert!(status.success());
    assert_eq!(
        fs::symlink_metadata(&runtime).unwrap().permissions().mode() & 0o7777,
        0o700
    );
    assert_eq!(
        fs::symlink_metadata(runtime.join("state"))
            .unwrap()
            .permissions()
            .mode()
            & 0o7777,
        0o600
    );
}

#[cfg(target_os = "linux")]
#[test]
fn private_runtime_creation_rejects_a_read_only_parent() {
    if rustix::process::geteuid().as_raw() == 0 {
        eprintln!("permission denial is not meaningful for effective uid 0");
        return;
    }
    let temporary = tempdir().expect("temporary directory");
    let parent = temporary.path().join("read-only");
    fs::create_dir(&parent).expect("parent");
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o500)).expect("read-only mode");
    let state = parent.join("state");
    let error = ensure_private_file(&state).expect_err("read-only parent must reject creation");
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).expect("restore mode");
    assert!(error.detail().contains("cannot create private file"));
    assert!(!state.exists());
}

#[cfg(target_os = "linux")]
#[test]
fn publication_rejects_symlink_and_read_only_faults_without_false_success() {
    let temporary = tempdir().expect("temporary directory");
    let parent = temporary.path().join("publication");
    fs::create_dir(&parent).expect("parent");
    let ordinary = parent.join("ordinary");
    let linked = parent.join("linked.staging");
    let destination = parent.join("published");
    fs::write(&ordinary, b"complete").expect("ordinary");
    symlink("ordinary", &linked).expect("staging symlink");
    let symlink_error = publish_file_no_replace(&linked, &destination)
        .expect_err("symlink staging must be rejected");
    assert_eq!(symlink_error.phase(), NoReplacePublicationPhase::Validation);
    assert_eq!(
        symlink_error.effect(),
        NoReplacePublicationEffect::NamesUnchanged
    );
    assert_eq!(fs::read(&ordinary).expect("ordinary bytes"), b"complete");
    assert!(linked.symlink_metadata().is_ok());
    assert!(!destination.exists());

    if rustix::process::geteuid().as_raw() == 0 {
        eprintln!("read-only publication denial is not meaningful for effective uid 0");
        return;
    }
    let staged = parent.join("ordinary.staging");
    fs::write(&staged, b"staged").expect("staged");
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o500)).expect("read-only mode");
    let permission_error = publish_file_no_replace(&staged, &destination)
        .expect_err("read-only parent must reject publication");
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).expect("restore mode");
    assert_eq!(
        permission_error.phase(),
        NoReplacePublicationPhase::NamespacePublication
    );
    assert_eq!(
        permission_error.effect(),
        NoReplacePublicationEffect::NamesUnchanged
    );
    assert_eq!(fs::read(&staged).expect("preserved source"), b"staged");
    assert!(!destination.exists());
}

#[cfg(target_os = "linux")]
#[test]
fn mutation_lock_excludes_another_open_description() {
    let temporary = tempdir().expect("temporary directory");
    let runtime = temporary.path().join("runtime");
    ensure_private_directory(&runtime).expect("runtime");
    let path = runtime.join("mutation.lock");
    let first = MutationLockGuard::acquire(&path).expect("first lock");
    assert!(MutationLockGuard::try_acquire(&path)
        .expect("contended observation")
        .is_none());
    drop(first);
    assert!(MutationLockGuard::try_acquire(&path)
        .expect("released observation")
        .is_some());
}

#[cfg(target_os = "linux")]
#[test]
fn mutation_lock_process_probe() {
    let Some(path) = std::env::var_os("VOLICORD_TEST_MUTATION_LOCK") else {
        return;
    };
    let guard =
        MutationLockGuard::try_acquire(std::path::Path::new(&path)).expect("process lock probe");
    let acquired = guard.is_some();
    let expected =
        std::env::var_os("VOLICORD_TEST_LOCK_EXPECTED").expect("expected state") == "acquired";
    assert_eq!(acquired, expected);
    if acquired {
        if let Some(ready) = std::env::var_os("VOLICORD_TEST_LOCK_HOLD_READY") {
            fs::write(ready, b"ready\n").expect("lock-holder readiness");
            loop {
                thread::sleep(Duration::from_secs(60));
            }
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn mutation_lock_excludes_a_separate_process() {
    let temporary = tempdir().expect("temporary directory");
    let runtime = temporary.path().join("runtime");
    ensure_private_directory(&runtime).expect("runtime");
    let path = runtime.join("mutation.lock");
    let parent = MutationLockGuard::acquire(&path).expect("parent lock");
    run_lock_probe(&path, "contended");
    drop(parent);
    run_lock_probe(&path, "acquired");
}

#[cfg(target_os = "linux")]
#[test]
fn mutation_lock_is_released_after_holder_process_termination() {
    let temporary = tempdir().expect("temporary directory");
    let runtime = temporary.path().join("runtime");
    ensure_private_directory(&runtime).expect("runtime");
    let path = runtime.join("mutation.lock");
    for iteration in 0..8 {
        let ready = runtime.join(format!("holder-{iteration}.ready"));
        let mut child = KillAndReap(
            Command::new(std::env::current_exe().expect("current test executable"))
                .args(["--exact", "mutation_lock_process_probe", "--nocapture"])
                .env("VOLICORD_TEST_MUTATION_LOCK", &path)
                .env("VOLICORD_TEST_LOCK_EXPECTED", "acquired")
                .env("VOLICORD_TEST_LOCK_HOLD_READY", &ready)
                .spawn()
                .expect("lock-holder process"),
        );
        let deadline = Instant::now() + Duration::from_secs(2);
        while !ready.exists() && Instant::now() < deadline {
            if let Some(status) = child.0.try_wait().expect("holder status") {
                panic!("lock holder exited before readiness: {status}");
            }
            thread::sleep(Duration::from_millis(5));
        }
        if !ready.exists() {
            panic!("lock holder did not become ready");
        }
        let contended = MutationLockGuard::try_acquire(&path);
        child.0.kill().expect("terminate lock holder");
        let status = child.0.wait().expect("reap lock holder");
        assert!(!status.success());
        assert!(contended.expect("contended observation").is_none());
        assert!(MutationLockGuard::try_acquire(&path)
            .expect("post-termination observation")
            .is_some());
    }
}

#[cfg(target_os = "linux")]
fn run_lock_probe(path: &std::path::Path, expected: &str) {
    let status = Command::new(std::env::current_exe().expect("current test executable"))
        .args(["--exact", "mutation_lock_process_probe", "--nocapture"])
        .env("VOLICORD_TEST_MUTATION_LOCK", path)
        .env("VOLICORD_TEST_LOCK_EXPECTED", expected)
        .status()
        .expect("lock probe process");
    assert!(status.success());
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
