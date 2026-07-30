use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use serde_json::json;
use tempfile::TempDir;
use volicord_types::product_path::ProductRelativePath;

use super::{
    bounded::{git_command, set_test_git_global_config},
    ContentIdentity, GitObjectIdentity, InvocationObservationPaths, ObservationUnavailableReason,
    ObserverLimits, ProductPathState, RegularFileContentEvidence, RepositoryDelta,
    RepositoryObservationCheckpoint, RepositoryObserver, RepositoryPathTransition,
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
        let global_config = directory.path().join("isolated-global.gitconfig");
        fs::write(&global_config, b"")?;
        set_test_git_global_config(&global_config);
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

    fn git_stdout(&self, arguments: &[&str]) -> Result<String, Box<dyn Error>> {
        let output = run_git_output(&self.root, arguments)?;
        Ok(String::from_utf8(output)?.trim().to_owned())
    }

    fn commit_all(&self, message: &str) -> Result<(), Box<dyn Error>> {
        self.git(&["add", "-A"])?;
        self.commit_staged(message)
    }

    fn commit_staged(&self, message: &str) -> Result<(), Box<dyn Error>> {
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
fn crlf_attribute_staging_and_commit_compare_in_the_canonical_git_domain() -> TestResult {
    let fixture = GitFixture::new()?;
    fixture.write(".gitattributes", b"tracked.txt text eol=crlf\n")?;
    fixture.write("tracked.txt", b"base\r\n")?;
    fixture.commit_all("crlf base")?;
    fixture.write("tracked.txt", b"pre-existing\r\nworktree bytes\r\n")?;
    let observer = fixture.observer()?;
    let before = observer.snapshot(&InvocationObservationPaths::default())?;
    let unchanged_bytes = fs::read(fixture.root.join("tracked.txt"))?;

    fixture.git(&["add", "tracked.txt"])?;
    let staged = observer.snapshot(&InvocationObservationPaths::default())?;
    assert_eq!(fs::read(fixture.root.join("tracked.txt"))?, unchanged_bytes);
    assert!(observer.delta(&before, &staged)?.is_empty());

    fixture.commit_staged("stage unchanged CRLF bytes")?;
    let committed = observer.snapshot(&InvocationObservationPaths::default())?;
    assert_eq!(fs::read(fixture.root.join("tracked.txt"))?, unchanged_bytes);
    assert!(observer.delta(&before, &committed)?.is_empty());

    fixture.write("tracked.txt", b"actual\r\ncontent change\r\n")?;
    let changed = observer.snapshot(&InvocationObservationPaths::default())?;
    assert_eq!(
        transition_paths(&observer.delta(&committed, &changed)?),
        ["tracked.txt"]
    );
    Ok(())
}

#[test]
fn core_autocrlf_staging_and_commit_preserve_unchanged_worktree_bytes() -> TestResult {
    let fixture = GitFixture::new()?;
    fixture.git(&["config", "core.autocrlf", "true"])?;
    fixture.write(".gitattributes", b"tracked.txt text\n")?;
    fixture.write("tracked.txt", b"base\r\n")?;
    fixture.commit_all("autocrlf base")?;
    fixture.write("tracked.txt", b"pre-existing\r\nautocrlf bytes\r\n")?;
    let observer = fixture.observer()?;
    let before = observer.snapshot(&InvocationObservationPaths::default())?;
    let unchanged_bytes = fs::read(fixture.root.join("tracked.txt"))?;

    fixture.git(&["add", "tracked.txt"])?;
    let staged = observer.snapshot(&InvocationObservationPaths::default())?;
    assert!(observer.delta(&before, &staged)?.is_empty());
    fixture.commit_staged("stage unchanged autocrlf bytes")?;
    let committed = observer.snapshot(&InvocationObservationPaths::default())?;
    assert_eq!(fs::read(fixture.root.join("tracked.txt"))?, unchanged_bytes);
    assert!(observer.delta(&before, &committed)?.is_empty());

    fixture.write("tracked.txt", b"actual\r\nautocrlf change\r\n")?;
    let changed = observer.snapshot(&InvocationObservationPaths::default())?;
    assert_eq!(
        transition_paths(&observer.delta(&committed, &changed)?),
        ["tracked.txt"]
    );
    Ok(())
}

#[test]
fn working_tree_encoding_uses_git_conversion_for_cross_source_comparison() -> TestResult {
    let fixture = GitFixture::new()?;
    fixture.write(
        ".gitattributes",
        b"tracked.txt text working-tree-encoding=UTF-16LE\n",
    )?;
    fixture.write("tracked.txt", &utf16le("base\n"))?;
    fixture.commit_all("encoded base")?;
    fixture.write("tracked.txt", &utf16le("pre-existing encoded bytes\n"))?;
    let observer = fixture.observer()?;
    let before = observer.snapshot(&InvocationObservationPaths::default())?;
    let unchanged_bytes = fs::read(fixture.root.join("tracked.txt"))?;

    fixture.git(&["add", "tracked.txt"])?;
    let staged = observer.snapshot(&InvocationObservationPaths::default())?;
    assert!(observer.delta(&before, &staged)?.is_empty());
    fixture.commit_staged("stage unchanged encoded bytes")?;
    let committed = observer.snapshot(&InvocationObservationPaths::default())?;
    assert_eq!(fs::read(fixture.root.join("tracked.txt"))?, unchanged_bytes);
    assert!(observer.delta(&before, &committed)?.is_empty());

    fixture.write("tracked.txt", &utf16le("actual encoded change\n"))?;
    let changed = observer.snapshot(&InvocationObservationPaths::default())?;
    assert_eq!(
        transition_paths(&observer.delta(&committed, &changed)?),
        ["tracked.txt"]
    );
    Ok(())
}

#[test]
fn clean_filter_preserves_source_domains_and_matches_the_committed_blob() -> TestResult {
    let fixture = GitFixture::new()?;
    fixture.write(".gitattributes", b"tracked.txt filter=volicord-normalize\n")?;
    fixture.git(&[
        "config",
        "filter.volicord-normalize.clean",
        "git stripspace",
    ])?;
    fixture.git(&[
        "config",
        "filter.volicord-normalize.smudge",
        "git stripspace",
    ])?;
    fixture.git(&["config", "filter.volicord-normalize.required", "true"])?;
    fixture.write("tracked.txt", b"base\n")?;
    fixture.commit_all("filtered base")?;
    fixture.write("tracked.txt", b"alpha\n\n")?;
    let observer = fixture.observer()?;
    let before = observer.snapshot(&InvocationObservationPaths::default())?;
    let before_evidence = regular_file_evidence(&before, "tracked.txt")?.clone();
    let unchanged_bytes = fs::read(fixture.root.join("tracked.txt"))?;

    fixture.git(&["add", "tracked.txt"])?;
    let staged = observer.snapshot(&InvocationObservationPaths::default())?;
    assert!(observer.delta(&before, &staged)?.is_empty());
    fixture.commit_staged("stage unchanged filtered bytes")?;
    let committed = observer.snapshot(&InvocationObservationPaths::default())?;
    assert_eq!(fs::read(fixture.root.join("tracked.txt"))?, unchanged_bytes);
    assert!(observer.delta(&before, &committed)?.is_empty());
    assert_eq!(
        before_evidence.canonical_git_blob().as_str(),
        fixture.git_stdout(&["rev-parse", "HEAD:tracked.txt"])?
    );

    let direct_paths =
        InvocationObservationPaths::new(vec![product_path("tracked.txt")?], Vec::new());
    let direct_before = observer.snapshot(&direct_paths)?;
    fixture.write("tracked.txt", b"alpha\n")?;
    let direct_after = observer.snapshot(&direct_paths)?;
    let before_direct_evidence = regular_file_evidence(&direct_before, "tracked.txt")?;
    let after_direct_evidence = regular_file_evidence(&direct_after, "tracked.txt")?;
    assert_eq!(
        before_direct_evidence.canonical_git_blob(),
        after_direct_evidence.canonical_git_blob()
    );
    assert_ne!(
        before_direct_evidence.exact_worktree_bytes(),
        after_direct_evidence.exact_worktree_bytes()
    );
    assert_eq!(
        transition_paths(&observer.delta(&direct_before, &direct_after)?),
        ["tracked.txt"]
    );

    let identical = observer.snapshot(&direct_paths)?;
    assert!(observer.delta(&direct_after, &identical)?.is_empty());
    fixture.write("tracked.txt", b"beta\n\n")?;
    let changed = observer.snapshot(&InvocationObservationPaths::default())?;
    assert_eq!(
        transition_paths(&observer.delta(&committed, &changed)?),
        ["tracked.txt"]
    );
    Ok(())
}

#[test]
fn regular_file_comparison_is_symmetric_across_worktree_and_tree_sources() -> TestResult {
    let canonical = GitObjectIdentity::parse("a".repeat(40))?;
    let different = GitObjectIdentity::parse("b".repeat(40))?;
    let worktree = ProductPathState::RegularFile {
        content_evidence: RegularFileContentEvidence::Worktree {
            exact_worktree_bytes: ContentIdentity::for_bytes(b"worktree"),
            canonical_git_blob: canonical.clone(),
        },
        executable: false,
    };
    let tree = ProductPathState::RegularFile {
        content_evidence: RegularFileContentEvidence::GitTree {
            canonical_git_blob: canonical,
        },
        executable: false,
    };
    let changed_tree = ProductPathState::RegularFile {
        content_evidence: RegularFileContentEvidence::GitTree {
            canonical_git_blob: different,
        },
        executable: false,
    };

    assert!(worktree.semantically_eq(&tree));
    assert!(tree.semantically_eq(&worktree));
    assert!(!worktree.semantically_eq(&changed_tree));
    assert!(!changed_tree.semantically_eq(&worktree));
    Ok(())
}

#[test]
fn strict_regular_file_evidence_and_semantic_delta_validation_reject_corruption() -> TestResult {
    let exact = ContentIdentity::for_bytes(b"same");
    let canonical = "a".repeat(40);
    let valid_worktree = json!({
        "kind": "regular_file",
        "content_evidence": {
            "source": "worktree",
            "exact_worktree_bytes": exact.as_str(),
            "canonical_git_blob": canonical,
        },
        "executable": false,
    });
    let missing_canonical = json!({
        "kind": "regular_file",
        "content_evidence": {
            "source": "worktree",
            "exact_worktree_bytes": exact.as_str(),
        },
        "executable": false,
    });
    let missing_exact_worktree_bytes = json!({
        "kind": "regular_file",
        "content_evidence": {
            "source": "worktree",
            "canonical_git_blob": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        },
        "executable": false,
    });
    let fabricated_tree_worktree_evidence = json!({
        "kind": "regular_file",
        "content_evidence": {
            "source": "git_tree",
            "exact_worktree_bytes": exact.as_str(),
            "canonical_git_blob": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        },
        "executable": false,
    });
    let malformed_object_id = json!({
        "kind": "regular_file",
        "content_evidence": {
            "source": "git_tree",
            "canonical_git_blob": "not-an-object-id",
        },
        "executable": false,
    });
    let noncanonical_object_id = json!({
        "kind": "regular_file",
        "content_evidence": {
            "source": "git_tree",
            "canonical_git_blob": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        },
        "executable": false,
    });
    let former_regular_file_shape = json!({
        "kind": "regular_file",
        "content": exact.as_str(),
        "executable": false,
    });

    assert!(serde_json::from_value::<ProductPathState>(valid_worktree.clone()).is_ok());
    assert!(serde_json::from_value::<ProductPathState>(missing_canonical).is_err());
    assert!(serde_json::from_value::<ProductPathState>(missing_exact_worktree_bytes).is_err());
    assert!(serde_json::from_value::<ProductPathState>(fabricated_tree_worktree_evidence).is_err());
    assert!(serde_json::from_value::<ProductPathState>(malformed_object_id).is_err());
    assert!(serde_json::from_value::<ProductPathState>(noncanonical_object_id).is_err());
    assert!(serde_json::from_value::<ProductPathState>(former_regular_file_shape).is_err());
    assert!(serde_json::from_value::<ProductPathState>(json!({
        "kind": "regular_file",
        "content_evidence": {
            "source": "git_tree",
            "canonical_git_blob": "b".repeat(64),
        },
        "executable": false,
    }))
    .is_ok());

    let semantic_no_op_transition = json!({
        "path": "tracked.txt",
        "before": valid_worktree,
        "after": {
            "kind": "regular_file",
            "content_evidence": {
                "source": "git_tree",
                "canonical_git_blob": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            },
            "executable": false,
        },
    });
    assert!(
        serde_json::from_value::<RepositoryPathTransition>(semantic_no_op_transition.clone())
            .is_err()
    );
    let semantic_no_op = json!({
        "transitions": [semantic_no_op_transition],
    });
    assert!(serde_json::from_value::<RepositoryDelta>(semantic_no_op).is_err());
    Ok(())
}

#[test]
fn required_clean_filter_failure_is_an_unavailable_observation() -> TestResult {
    let fixture = GitFixture::new()?;
    fixture.write(".gitattributes", b"tracked.txt filter=volicord-failing\n")?;
    fixture.git(&[
        "config",
        "filter.volicord-failing.clean",
        "git cat-file -e 0000000000000000000000000000000000000000",
    ])?;
    fixture.git(&["config", "filter.volicord-failing.required", "true"])?;
    fixture.commit_all("failing filter configuration")?;
    fixture.write("tracked.txt", b"must fail conversion\n")?;
    let observer = fixture.observer()?;

    assert_eq!(
        observer
            .snapshot(&InvocationObservationPaths::default())
            .expect_err("required filter failure must not produce a snapshot")
            .reason(),
        ObservationUnavailableReason::GitCommandFailed
    );
    Ok(())
}

#[test]
fn injected_malformed_canonical_git_identities_are_unavailable() {
    use super::path_state::parse_canonical_git_identity;

    for output in [
        b"".as_slice(),
        b"abc\nextra\n".as_slice(),
        b"ABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCD\n".as_slice(),
        b"not-an-object-identity\n".as_slice(),
        b"\xff\n".as_slice(),
    ] {
        let error = parse_canonical_git_identity(output)
            .expect_err("malformed or noncanonical Git identity must be unavailable");
        assert_eq!(
            error.reason(),
            ObservationUnavailableReason::GitObjectUnavailable
        );
    }
}

#[test]
fn nonterminating_clean_filter_is_bounded_by_the_process_timeout() -> TestResult {
    let fixture = GitFixture::new()?;
    fixture.write(".gitattributes", b"tracked.txt filter=volicord-hanging\n")?;
    let executable = env::current_exe()?;
    let executable = executable.to_string_lossy().replace('\\', "/");
    let filter = format!(
        "\"{}\" --ignored --exact repository_observation::tests::nonterminating_filter_process --nocapture",
        executable.replace('"', "\\\"")
    );
    fixture.git(&["config", "filter.volicord-hanging.clean", &filter])?;
    fixture.git(&["config", "filter.volicord-hanging.required", "true"])?;
    fixture.commit_all("hanging filter configuration")?;
    fixture.write("tracked.txt", b"must time out\n")?;
    let observer = RepositoryObserver::new(
        &fixture.root,
        ObserverLimits::default().with_max_process_duration(Duration::from_millis(150)),
    )?;

    assert_eq!(
        observer
            .snapshot(&InvocationObservationPaths::default())
            .expect_err("nonterminating filter must not produce a snapshot")
            .reason(),
        ObservationUnavailableReason::ProcessTimeout
    );
    Ok(())
}

#[test]
#[ignore = "executed only as a contained clean-filter process"]
fn nonterminating_filter_process() {
    thread::sleep(Duration::from_secs(30));
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
    let output = run_git_output(repository_root, arguments)?;
    let _ = output;
    Ok(())
}

fn run_git_output(repository_root: &Path, arguments: &[&str]) -> Result<Vec<u8>, Box<dyn Error>> {
    let output = git_command(repository_root).args(arguments).output()?;
    if output.status.success() {
        return Ok(output.stdout);
    }
    Err(format!(
        "git {:?} failed: {}",
        arguments,
        String::from_utf8_lossy(&output.stderr).trim()
    )
    .into())
}

fn utf16le(value: &str) -> Vec<u8> {
    value
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>()
}

fn regular_file_evidence<'a>(
    snapshot: &'a super::RepositoryObservationSnapshot,
    path: &str,
) -> Result<&'a RegularFileContentEvidence, Box<dyn Error>> {
    let path = product_path(path)?;
    match snapshot.observed_states().get(&path) {
        Some(ProductPathState::RegularFile {
            content_evidence, ..
        }) => Ok(content_evidence),
        _ => Err(format!("snapshot did not directly observe regular file {path}").into()),
    }
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
