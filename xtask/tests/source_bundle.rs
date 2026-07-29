#![cfg(unix)]

use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

#[test]
fn canonical_command_uses_committed_tree_content_and_preserves_git_modes() {
    let repository = fixture_repository();
    let output = repository.path().join("source.zip");
    fs::write(repository.path().join(".git/private"), b"Git metadata\n")
        .expect("Git metadata fixture");
    fs::write(repository.path().join("runtime.sqlite"), b"runtime\n")
        .expect("untracked database fixture");
    fs::write(repository.path().join("local.log"), b"log\n").expect("untracked log fixture");
    fs::write(repository.path().join("previous.zip"), b"archive\n")
        .expect("untracked archive fixture");

    let command = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .current_dir(repository.path())
        .args([
            "source-bundle",
            "--output",
            output.to_str().expect("UTF-8 output"),
        ])
        .output()
        .expect("run source-bundle command");
    assert!(
        command.status.success(),
        "source-bundle command failed:\n{}",
        String::from_utf8_lossy(&command.stderr)
    );

    let report =
        xtask::validate_source_bundle(repository.path(), &output, None).expect("validate bundle");
    assert_eq!(report.entry_count(), 5);

    let extraction = repository.path().join("extracted");
    let unzip = Command::new("unzip")
        .args(["-q", output.to_str().expect("UTF-8 output"), "-d"])
        .arg(&extraction)
        .output()
        .expect("extract source bundle");
    assert!(
        unzip.status.success(),
        "unzip failed:\n{}",
        String::from_utf8_lossy(&unzip.stderr)
    );

    assert!(!extraction.join(".git").exists());
    assert!(!extraction.join("runtime.sqlite").exists());
    assert!(!extraction.join("local.log").exists());
    assert!(!extraction.join("previous.zip").exists());
    assert_eq!(
        fs::read(extraction.join("regular.txt")).expect("regular content"),
        git_blob(repository.path(), "HEAD:regular.txt")
    );
    assert_eq!(
        fs::metadata(extraction.join("regular.txt"))
            .expect("regular metadata")
            .permissions()
            .mode()
            & 0o777,
        0o644
    );
    assert_eq!(
        fs::metadata(extraction.join("scripts/run.sh"))
            .expect("executable metadata")
            .permissions()
            .mode()
            & 0o777,
        0o755
    );
    assert_eq!(
        fs::read_link(extraction.join("regular-link")).expect("symbolic link"),
        PathBuf::from("regular.txt")
    );
}

#[test]
fn repeated_generation_is_byte_for_byte_deterministic() {
    let repository = fixture_repository();
    let first = repository.path().join("first.zip");
    let second = repository.path().join("second.zip");

    let first_report =
        xtask::create_source_bundle(repository.path(), &first, None).expect("first bundle");
    let second_report =
        xtask::create_source_bundle(repository.path(), &second, None).expect("second bundle");

    assert_eq!(first_report.commit(), second_report.commit());
    assert_eq!(first_report.tree(), second_report.tree());
    assert_eq!(
        fs::read(first).expect("first bytes"),
        fs::read(second).expect("second bytes")
    );
}

#[test]
fn default_head_rejects_tracked_changes_and_explicit_commit_remains_available() {
    let repository = fixture_repository();
    fs::write(repository.path().join("regular.txt"), b"modified\n").expect("tracked modification");

    let error = xtask::create_source_bundle(
        repository.path(),
        &repository.path().join("dirty.zip"),
        None,
    )
    .expect_err("dirty default HEAD must fail");
    assert!(error
        .to_string()
        .contains("tracked index or working-tree changes"));

    let explicit = repository.path().join("explicit.zip");
    xtask::create_source_bundle(repository.path(), &explicit, Some("HEAD"))
        .expect("explicit selected commit");
    xtask::validate_source_bundle(repository.path(), &explicit, Some("HEAD"))
        .expect("validate explicit commit bundle");
}

#[test]
fn complete_repository_bundle_validates_against_selected_head() {
    let root = repository_root();
    let temporary = tempfile::tempdir().expect("temporary output");
    let output = temporary.path().join("volicord-source.zip");

    let created =
        xtask::create_source_bundle(root, &output, Some("HEAD")).expect("complete source bundle");
    let validated =
        xtask::validate_source_bundle(root, &output, Some("HEAD")).expect("complete validation");

    assert_eq!(created, validated);
    assert!(created.entry_count() > 100);
}

fn fixture_repository() -> TempDir {
    let temporary = tempfile::tempdir().expect("fixture repository");
    git(temporary.path(), &["init", "-q"]);
    git(
        temporary.path(),
        &["config", "user.name", "Source Bundle Test"],
    );
    git(
        temporary.path(),
        &["config", "user.email", "source-bundle@example.invalid"],
    );

    fs::create_dir(temporary.path().join("scripts")).expect("scripts directory");
    fs::write(temporary.path().join("regular.txt"), b"regular content\n").expect("regular file");
    fs::write(
        temporary.path().join("scripts/run.sh"),
        b"#!/bin/sh\nexit 0\n",
    )
    .expect("executable file");
    let mut executable_permissions = fs::metadata(temporary.path().join("scripts/run.sh"))
        .expect("executable metadata")
        .permissions();
    executable_permissions.set_mode(0o755);
    fs::set_permissions(
        temporary.path().join("scripts/run.sh"),
        executable_permissions,
    )
    .expect("executable permissions");
    symlink("regular.txt", temporary.path().join("regular-link")).expect("symbolic link");
    fs::write(temporary.path().join("nested.txt"), b"another file\n").expect("second regular file");

    git(temporary.path(), &["add", "."]);
    git(temporary.path(), &["commit", "-q", "-m", "fixture"]);
    temporary
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {} failed:\n{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_blob(root: &Path, object: &str) -> Vec<u8> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["cat-file", "blob", object])
        .output()
        .expect("read Git blob");
    assert!(output.status.success());
    output.stdout
}

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is below repository root")
}
