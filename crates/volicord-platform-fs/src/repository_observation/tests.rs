use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use tempfile::TempDir;
use volicord_types::product_path::ProductRelativePath;

use super::{
    InvocationObservationPaths, ObservationUnavailableReason, ObserverLimits, ProductPathState,
    RepositoryObservationCheckpoint, RepositoryObserver,
};

type TestResult = Result<(), Box<dyn Error>>;

struct GitFixture {
    _directory: TempDir,
    root: PathBuf,
}

impl GitFixture {
    fn new() -> Result<Self, Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("repository");
        fs::create_dir(&root)?;
        run_git(&root, &["init", "-q"])?;
        run_git(&root, &["symbolic-ref", "HEAD", "refs/heads/main"])?;
        run_git(&root, &["config", "user.name", "Volicord Observer Test"])?;
        run_git(
            &root,
            &["config", "user.email", "observer-test@example.invalid"],
        )?;
        run_git(&root, &["config", "core.autocrlf", "false"])?;
        Ok(Self {
            _directory: directory,
            root,
        })
    }

    fn with_base_file() -> Result<Self, Box<dyn Error>> {
        let fixture = Self::new()?;
        fixture.write("tracked.txt", b"base\n")?;
        fixture.commit_all("base")?;
        Ok(fixture)
    }

    fn write(&self, path: &str, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
        let absolute = self.root.join(path);
        if let Some(parent) = absolute.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(absolute, bytes)?;
        Ok(())
    }

    fn remove(&self, path: &str) -> Result<(), Box<dyn Error>> {
        fs::remove_file(self.root.join(path))?;
        Ok(())
    }

    fn git(&self, arguments: &[&str]) -> Result<(), Box<dyn Error>> {
        run_git(&self.root, arguments)
    }

    fn commit_all(&self, message: &str) -> Result<(), Box<dyn Error>> {
        self.git(&["add", "-A"])?;
        self.git(&[
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            message,
        ])
    }

    fn observer(&self) -> Result<RepositoryObserver, Box<dyn Error>> {
        Ok(RepositoryObserver::new(
            &self.root,
            ObserverLimits::default(),
        )?)
    }
}

#[test]
fn identical_snapshots_always_produce_an_empty_delta() -> TestResult {
    let fixture = GitFixture::with_base_file()?;
    let observer = fixture.observer()?;
    let first = observer.snapshot(&InvocationObservationPaths::default())?;
    let second = observer.snapshot(&InvocationObservationPaths::default())?;

    assert_eq!(first, second);
    assert!(observer.delta(&first, &second)?.is_empty());
    let delta = observer.delta(&first, &first)?;

    assert!(delta.is_empty());
    Ok(())
}

#[test]
fn unchanged_pre_existing_dirty_tracked_file_produces_no_transition() -> TestResult {
    let fixture = GitFixture::with_base_file()?;
    fixture.write("tracked.txt", b"user change\n")?;
    let observer = fixture.observer()?;

    let before = observer.snapshot(&InvocationObservationPaths::default())?;
    let after = observer.snapshot(&InvocationObservationPaths::default())?;

    assert!(observer.delta(&before, &after)?.is_empty());
    Ok(())
}

#[test]
fn pre_existing_dirty_tracked_file_modified_again_produces_one_transition() -> TestResult {
    let fixture = GitFixture::with_base_file()?;
    fixture.write("tracked.txt", b"first user change\n")?;
    let observer = fixture.observer()?;
    let before = observer.snapshot(&InvocationObservationPaths::default())?;

    fixture.write("tracked.txt", b"second user change\n")?;
    let after = observer.snapshot(&InvocationObservationPaths::default())?;
    let delta = observer.delta(&before, &after)?;

    assert_eq!(transition_paths(&delta), ["tracked.txt"]);
    assert!(matches!(
        delta.transitions()[0].before(),
        ProductPathState::RegularFile { .. }
    ));
    assert!(matches!(
        delta.transitions()[0].after(),
        ProductPathState::RegularFile { .. }
    ));
    Ok(())
}

#[test]
fn pre_existing_untracked_file_distinguishes_unchanged_changed_and_deleted() -> TestResult {
    let fixture = GitFixture::with_base_file()?;
    fixture.write("untracked.txt", b"first\n")?;
    let observer = fixture.observer()?;
    let first = observer.snapshot(&InvocationObservationPaths::default())?;
    let unchanged = observer.snapshot(&InvocationObservationPaths::default())?;
    assert!(observer.delta(&first, &unchanged)?.is_empty());

    fixture.write("untracked.txt", b"second\n")?;
    let changed = observer.snapshot(&InvocationObservationPaths::default())?;
    assert_eq!(
        transition_paths(&observer.delta(&first, &changed)?),
        ["untracked.txt"]
    );

    fixture.remove("untracked.txt")?;
    let deleted = observer.snapshot(&InvocationObservationPaths::default())?;
    let deletion = observer.delta(&changed, &deleted)?;
    assert_eq!(transition_paths(&deletion), ["untracked.txt"]);
    assert_eq!(deletion.transitions()[0].after(), &ProductPathState::Absent);
    Ok(())
}

#[test]
fn typed_invocation_hint_preserves_an_ignored_pre_existing_path_state() -> TestResult {
    let fixture = GitFixture::with_base_file()?;
    fixture.write(".gitignore", b"ignored.txt\n")?;
    fixture.commit_all("ignore fixture path")?;
    fixture.write("ignored.txt", b"first ignored bytes\n")?;
    let observer = fixture.observer()?;
    let paths = InvocationObservationPaths::new(Vec::new(), vec![product_path("ignored.txt")?]);
    let before = observer.snapshot(&paths)?;

    fixture.write("ignored.txt", b"second ignored bytes\n")?;
    let after = observer.snapshot(&paths)?;

    assert_eq!(
        transition_paths(&observer.delta(&before, &after)?),
        ["ignored.txt"]
    );
    Ok(())
}

#[test]
fn untracked_creation_and_clean_tracked_modification_are_net_transitions() -> TestResult {
    let fixture = GitFixture::with_base_file()?;
    let observer = fixture.observer()?;
    let before = observer.snapshot(&InvocationObservationPaths::default())?;

    fixture.write("created.txt", b"created\n")?;
    fixture.write("tracked.txt", b"modified\n")?;
    let after = observer.snapshot(&InvocationObservationPaths::default())?;
    let delta = observer.delta(&before, &after)?;

    assert_eq!(transition_paths(&delta), ["created.txt", "tracked.txt"]);
    assert_eq!(delta.transitions()[0].before(), &ProductPathState::Absent);
    Ok(())
}

#[test]
fn committed_tracked_creation_and_deletion_use_tree_state() -> TestResult {
    let fixture = GitFixture::with_base_file()?;
    let observer = fixture.observer()?;
    let before_create = observer.snapshot(&InvocationObservationPaths::default())?;

    fixture.write("created.txt", b"tracked creation\n")?;
    fixture.commit_all("create tracked file")?;
    let after_create = observer.snapshot(&InvocationObservationPaths::default())?;
    let creation = observer.delta(&before_create, &after_create)?;
    assert_eq!(transition_paths(&creation), ["created.txt"]);
    assert_eq!(
        creation.transitions()[0].before(),
        &ProductPathState::Absent
    );

    fixture.remove("created.txt")?;
    fixture.commit_all("delete tracked file")?;
    let after_delete = observer.snapshot(&InvocationObservationPaths::default())?;
    let deletion = observer.delta(&after_create, &after_delete)?;
    assert_eq!(transition_paths(&deletion), ["created.txt"]);
    assert_eq!(deletion.transitions()[0].after(), &ProductPathState::Absent);
    Ok(())
}

#[test]
fn rename_is_the_corresponding_delete_and_create_transition_pair() -> TestResult {
    let fixture = GitFixture::with_base_file()?;
    let observer = fixture.observer()?;
    let before = observer.snapshot(&InvocationObservationPaths::default())?;

    fixture.git(&["mv", "tracked.txt", "renamed.txt"])?;
    let after = observer.snapshot(&InvocationObservationPaths::default())?;
    let delta = observer.delta(&before, &after)?;

    assert_eq!(transition_paths(&delta), ["renamed.txt", "tracked.txt"]);
    assert_eq!(delta.transitions()[0].before(), &ProductPathState::Absent);
    assert_eq!(delta.transitions()[1].after(), &ProductPathState::Absent);
    Ok(())
}

#[cfg(unix)]
#[test]
fn executable_bit_and_symbolic_link_target_changes_are_observed() -> TestResult {
    use std::os::unix::fs::{symlink, PermissionsExt};

    let fixture = GitFixture::new()?;
    fixture.write("script.sh", b"#!/bin/sh\nexit 0\n")?;
    fixture.write("target-a", b"a\n")?;
    fixture.write("target-b", b"b\n")?;
    symlink("target-a", fixture.root.join("link"))?;
    fixture.commit_all("executable and link base")?;
    fixture.git(&["config", "core.filemode", "false"])?;
    let observer = fixture.observer()?;
    let before = observer.snapshot(&InvocationObservationPaths::default())?;

    let mut permissions = fs::metadata(fixture.root.join("script.sh"))?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(fixture.root.join("script.sh"), permissions)?;
    fixture.remove("link")?;
    symlink("target-b", fixture.root.join("link"))?;
    let after = observer.snapshot(&InvocationObservationPaths::default())?;
    let delta = observer.delta(&before, &after)?;

    assert_eq!(transition_paths(&delta), ["link", "script.sh"]);
    assert!(matches!(
        delta.transitions()[1].before(),
        ProductPathState::RegularFile {
            executable: false,
            ..
        }
    ));
    assert!(matches!(
        delta.transitions()[1].after(),
        ProductPathState::RegularFile {
            executable: true,
            ..
        }
    ));
    Ok(())
}

#[test]
fn modification_committed_before_post_snapshot_remains_visible() -> TestResult {
    let fixture = GitFixture::with_base_file()?;
    let observer = fixture.observer()?;
    let before = observer.snapshot(&InvocationObservationPaths::default())?;

    fixture.write("tracked.txt", b"tool change\n")?;
    fixture.commit_all("tool commit")?;
    let after = observer.snapshot(&InvocationObservationPaths::default())?;

    assert_eq!(
        transition_paths(&observer.delta(&before, &after)?),
        ["tracked.txt"]
    );
    Ok(())
}

#[test]
fn staging_or_committing_existing_worktree_bytes_produces_no_delta() -> TestResult {
    let fixture = GitFixture::with_base_file()?;
    fixture.write("tracked.txt", b"pre-existing user bytes\n")?;
    let observer = fixture.observer()?;
    let before = observer.snapshot(&InvocationObservationPaths::default())?;

    fixture.git(&["add", "tracked.txt"])?;
    let staged = observer.snapshot(&InvocationObservationPaths::default())?;
    assert!(observer.delta(&before, &staged)?.is_empty());

    fixture.commit_all("record existing bytes")?;
    let committed = observer.snapshot(&InvocationObservationPaths::default())?;
    assert!(observer.delta(&before, &committed)?.is_empty());
    Ok(())
}

#[test]
fn restore_removing_a_pre_existing_user_change_is_a_transition() -> TestResult {
    let fixture = GitFixture::with_base_file()?;
    fixture.write("tracked.txt", b"user bytes\n")?;
    let observer = fixture.observer()?;
    let before = observer.snapshot(&InvocationObservationPaths::default())?;

    fixture.git(&["restore", "tracked.txt"])?;
    let after = observer.snapshot(&InvocationObservationPaths::default())?;

    assert_eq!(
        transition_paths(&observer.delta(&before, &after)?),
        ["tracked.txt"]
    );
    Ok(())
}

#[test]
fn branch_movement_that_changes_product_files_produces_a_delta() -> TestResult {
    let fixture = GitFixture::with_base_file()?;
    fixture.git(&["switch", "-q", "-c", "other"])?;
    fixture.write("tracked.txt", b"other branch\n")?;
    fixture.commit_all("other branch")?;
    fixture.git(&["switch", "-q", "main"])?;
    let observer = fixture.observer()?;
    let before = observer.snapshot(&InvocationObservationPaths::default())?;

    fixture.git(&["switch", "-q", "other"])?;
    let after = observer.snapshot(&InvocationObservationPaths::default())?;

    assert_eq!(
        transition_paths(&observer.delta(&before, &after)?),
        ["tracked.txt"]
    );
    Ok(())
}

#[test]
fn head_movement_with_an_identical_tree_produces_no_delta() -> TestResult {
    let fixture = GitFixture::with_base_file()?;
    let observer = fixture.observer()?;
    let before = observer.snapshot(&InvocationObservationPaths::default())?;

    fixture.commit_all("empty commit")?;
    let after = observer.snapshot(&InvocationObservationPaths::default())?;

    assert_ne!(
        before.coordinate().head_oid(),
        after.coordinate().head_oid()
    );
    assert_eq!(
        before.coordinate().tree_oid(),
        after.coordinate().tree_oid()
    );
    assert!(observer.delta(&before, &after)?.is_empty());
    Ok(())
}

#[test]
fn initialized_clean_gitlink_state_tracks_checked_out_commit() -> TestResult {
    let nested = GitFixture::with_base_file()?;
    let fixture = GitFixture::new()?;
    fixture.git(&[
        "-c",
        "protocol.file.allow=always",
        "submodule",
        "add",
        "-q",
        nested.root.to_str().ok_or("non-UTF-8 nested path")?,
        "dependency",
    ])?;
    fixture.commit_all("add gitlink")?;
    let observer = fixture.observer()?;
    let before = observer.snapshot(&InvocationObservationPaths::default())?;

    let nested_checkout = fixture.root.join("dependency");
    fs::write(nested_checkout.join("tracked.txt"), b"next nested commit\n")?;
    run_git(&nested_checkout, &["add", "tracked.txt"])?;
    run_git(
        &nested_checkout,
        &[
            "-c",
            "user.name=Nested Test",
            "-c",
            "user.email=nested@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-q",
            "-m",
            "nested commit",
        ],
    )?;
    let after = observer.snapshot(&InvocationObservationPaths::default())?;
    let delta = observer.delta(&before, &after)?;

    assert_eq!(transition_paths(&delta), ["dependency"]);
    assert!(matches!(
        delta.transitions()[0].before(),
        ProductPathState::Gitlink { .. }
    ));
    assert!(matches!(
        delta.transitions()[0].after(),
        ProductPathState::Gitlink { .. }
    ));
    Ok(())
}

#[test]
fn changing_candidate_content_during_capture_is_explicitly_unstable() -> TestResult {
    let fixture = GitFixture::with_base_file()?;
    fixture.write("tracked.txt", b"first dirty state\n")?;
    let observer = RepositoryObserver::new(
        &fixture.root,
        ObserverLimits::default().with_max_stability_attempts(2),
    )?;

    let result = observer.snapshot_with_stability_hook(
        &InvocationObservationPaths::default(),
        |attempt, root| {
            let bytes = if attempt == 0 {
                b"second dirty state\n".as_slice()
            } else {
                b"third dirty state\n".as_slice()
            };
            let _ = fs::write(root.join("tracked.txt"), bytes);
        },
    );

    assert_eq!(
        result.expect_err("capture must remain unstable").reason(),
        ObservationUnavailableReason::UnstableRepository
    );
    Ok(())
}

#[test]
fn output_path_hash_and_file_size_limits_fail_closed() -> TestResult {
    let fixture = GitFixture::with_base_file()?;
    fixture.write("one.txt", b"0123456789")?;
    fixture.write("two.txt", b"abcdefghij")?;

    let output_observer = RepositoryObserver::new(
        &fixture.root,
        ObserverLimits::default().with_max_git_output_bytes(1),
    )?;
    assert_eq!(
        output_observer
            .snapshot(&InvocationObservationPaths::default())
            .expect_err("Git output must exceed one byte")
            .reason(),
        ObservationUnavailableReason::GitOutputLimitExceeded
    );

    let path_observer = RepositoryObserver::new(
        &fixture.root,
        ObserverLimits::default().with_max_candidate_paths(1),
    )?;
    assert_eq!(
        path_observer
            .snapshot(&InvocationObservationPaths::default())
            .expect_err("two candidate paths must exceed the path limit")
            .reason(),
        ObservationUnavailableReason::CandidatePathLimitExceeded
    );

    let hash_observer = RepositoryObserver::new(
        &fixture.root,
        ObserverLimits::default().with_max_total_hashed_bytes(25),
    )?;
    assert_eq!(
        hash_observer
            .snapshot(&InvocationObservationPaths::default())
            .expect_err("double stable observation must exceed the hash limit")
            .reason(),
        ObservationUnavailableReason::TotalHashBytesLimitExceeded
    );

    let file_observer = RepositoryObserver::new(
        &fixture.root,
        ObserverLimits::default().with_max_file_bytes(5),
    )?;
    assert_eq!(
        file_observer
            .snapshot(&InvocationObservationPaths::default())
            .expect_err("a ten-byte file must exceed the file limit")
            .reason(),
        ObservationUnavailableReason::FileSizeLimitExceeded
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn invocation_path_resolving_outside_repository_is_unavailable() -> TestResult {
    use std::os::unix::fs::symlink;

    let fixture = GitFixture::with_base_file()?;
    let outside = fixture
        .root
        .parent()
        .ok_or("repository has no parent")?
        .join("outside.txt");
    fs::write(&outside, b"outside\n")?;
    symlink(&outside, fixture.root.join("escape"))?;
    let observer = fixture.observer()?;
    let paths = InvocationObservationPaths::new(vec![product_path("escape")?], Vec::new());

    assert_eq!(
        observer
            .snapshot(&paths)
            .expect_err("external link must fail containment")
            .reason(),
        ObservationUnavailableReason::PathOutsideRepository
    );
    Ok(())
}

#[test]
fn canonical_serialization_and_contract_digests_are_deterministic_and_bounded() -> TestResult {
    let fixture = GitFixture::with_base_file()?;
    let paths = InvocationObservationPaths::new(vec![product_path("tracked.txt")?], Vec::new());
    let observer = fixture.observer()?;
    let first = observer.snapshot(&paths)?;
    let second = observer.snapshot(&paths)?;

    assert_eq!(first.canonical_bytes()?, second.canonical_bytes()?);
    assert_eq!(first.semantic_digest()?, second.semantic_digest()?);
    assert_eq!(first.contract_digest(), observer.contract_digest());
    let other = RepositoryObserver::new(
        &fixture.root,
        ObserverLimits::default().with_max_candidate_paths(64),
    )?;
    assert_ne!(observer.contract_digest(), other.contract_digest());

    let depth_error = RepositoryObserver::new(
        &fixture.root,
        ObserverLimits::default().with_max_serialization_depth(1),
    )
    .expect_err("insufficient serialization depth must fail");
    assert_eq!(
        depth_error.reason(),
        ObservationUnavailableReason::SerializationDepthLimitExceeded
    );
    let size_observer = RepositoryObserver::new(
        &fixture.root,
        ObserverLimits::default().with_max_serialized_bytes(8),
    )?;
    assert_eq!(
        size_observer
            .snapshot(&paths)
            .expect_err("typed input must exceed eight serialized bytes")
            .reason(),
        ObservationUnavailableReason::SerializationSizeLimitExceeded
    );
    Ok(())
}

fn run_git(repository_root: &Path, arguments: &[&str]) -> Result<(), Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository_root)
        .args(arguments)
        .output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "git {:?} failed: {}",
        arguments,
        String::from_utf8_lossy(&output.stderr).trim()
    )
    .into())
}

fn product_path(value: &str) -> Result<ProductRelativePath, Box<dyn Error>> {
    Ok(ProductRelativePath::parse(value.to_owned())?)
}

#[test]
fn checkpoint_restores_only_under_the_same_observer_contract() -> Result<(), Box<dyn Error>> {
    let repository = GitFixture::new()?;
    repository.write("tracked.txt", b"before")?;
    repository.commit_all("initial")?;
    let observer = repository.observer()?;
    let snapshot = observer.snapshot(&InvocationObservationPaths::new(
        vec![product_path("tracked.txt")?],
        Vec::new(),
    ))?;
    let checkpoint = snapshot.checkpoint();
    let checkpoint_json = serde_json::to_value(&checkpoint)?;
    let checkpoint: RepositoryObservationCheckpoint = serde_json::from_value(checkpoint_json)?;

    assert_eq!(observer.restore_checkpoint(checkpoint.clone())?, snapshot);

    let incompatible = RepositoryObserver::new(
        &repository.root,
        ObserverLimits::default().with_max_candidate_paths(8),
    )?;
    let error = incompatible
        .restore_checkpoint(checkpoint)
        .expect_err("different observer limits must reject the checkpoint");
    assert_eq!(
        error.reason(),
        ObservationUnavailableReason::ObserverContractMismatch
    );
    Ok(())
}

fn transition_paths<const N: usize>(delta: &super::RepositoryDelta) -> [&str; N] {
    let paths = delta
        .transitions()
        .iter()
        .map(|transition| transition.path().as_str())
        .collect::<Vec<_>>();
    let observed = paths.len();
    paths
        .try_into()
        .unwrap_or_else(|_| panic!("expected {N} transitions, observed {observed}"))
}
