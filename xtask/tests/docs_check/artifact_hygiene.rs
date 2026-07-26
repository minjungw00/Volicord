use super::*;
use std::process::Command;

#[test]
fn reports_tracked_files_excluded_by_repository_artifact_rules() {
    let fixture = valid_fixture();
    write(fixture.path(), ".gitignore", "*.sqlite\n");
    write(fixture.path(), "state.sqlite", "disposable state\n");

    git(fixture.path(), &["init", "--quiet"]);
    git(fixture.path(), &["add", ".gitignore"]);
    git(fixture.path(), &["add", "--force", "state.sqlite"]);

    let report = report(fixture.path());
    let errors = category_errors(&report, "artifact_hygiene.tracked_ignored");

    assert_eq!(errors.len(), 1, "{:#?}", report.issues());
    assert_eq!(errors[0].path(), "state.sqlite");
    assert!(
        errors[0].message().contains(".gitignore"),
        "{:#?}",
        report.issues()
    );
}

#[test]
fn accepts_tracked_files_allowed_by_repository_artifact_rules() {
    let fixture = valid_fixture();
    write(fixture.path(), ".gitignore", "*.sqlite\n");
    write(fixture.path(), "src/example.rs", "pub fn example() {}\n");

    git(fixture.path(), &["init", "--quiet"]);
    git(fixture.path(), &["add", ".gitignore", "src/example.rs"]);

    let report = report(fixture.path());

    assert!(
        category_errors(&report, "artifact_hygiene.tracked_ignored").is_empty(),
        "{:#?}",
        report.issues()
    );
}

fn git(root: &Path, arguments: &[&str]) {
    let output = Command::new("git")
        .current_dir(root)
        .args(arguments)
        .output()
        .expect("git command should start");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        arguments,
        String::from_utf8_lossy(&output.stderr)
    );
}
