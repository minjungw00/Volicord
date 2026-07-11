use std::fs;
use std::path::Path;

use tempfile::TempDir;

fn workspace_fixture() -> TempDir {
    let temp = tempfile::tempdir().expect("tempdir");
    write(
        temp.path(),
        "Cargo.toml",
        r#"[workspace]
members = ["crates/example", "xtask"]

[workspace.package]
version = "1.2.3"
"#,
    );
    write(
        temp.path(),
        "crates/example/Cargo.toml",
        r#"[package]
name = "example"
version.workspace = true
"#,
    );
    write(
        temp.path(),
        "xtask/Cargo.toml",
        r#"[package]
name = "xtask"
version.workspace = true
"#,
    );
    temp
}

fn write(root: &Path, path: &str, contents: &str) {
    let destination = root.join(path);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).expect("create fixture directory");
    }
    fs::write(destination, contents).expect("write fixture file");
}

#[test]
fn accepts_inherited_workspace_versions_and_matching_tag() {
    let fixture = workspace_fixture();

    let report = xtask::run_release_version_check(fixture.path(), Some("v1.2.3"))
        .expect("matching release version");

    assert_eq!(report.workspace_version(), "1.2.3");
    assert_eq!(report.member_package_count(), 2);
    assert_eq!(report.checked_tag(), Some("v1.2.3"));
}

#[test]
fn rejects_a_tag_that_does_not_exactly_match_workspace_version() {
    let fixture = workspace_fixture();

    let error = xtask::run_release_version_check(fixture.path(), Some("v1.2.4"))
        .expect_err("mismatched tag must fail");

    assert!(error.to_string().contains("expected \"v1.2.3\""));
}

#[test]
fn rejects_workspace_member_with_independent_version() {
    let fixture = workspace_fixture();
    write(
        fixture.path(),
        "crates/example/Cargo.toml",
        r#"[package]
name = "example"
version = "1.2.3"
"#,
    );

    let error = xtask::run_release_version_check(fixture.path(), None)
        .expect_err("independent member version must fail");

    assert!(error
        .to_string()
        .contains("example in crates/example/Cargo.toml must set version.workspace = true"));
}
