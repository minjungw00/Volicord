use std::process::Command;

#[test]
fn top_level_help_matches_snapshot() {
    assert_help_snapshot(&["--help"], include_str!("snapshots/xtask-help.txt"));
}

#[test]
fn validation_help_matches_snapshot() {
    assert_help_snapshot(
        &["validate", "--help"],
        include_str!("snapshots/xtask-validate-help.txt"),
    );
}

#[test]
fn source_bundle_help_matches_snapshot() {
    assert_help_snapshot(
        &["source-bundle", "--help"],
        include_str!("snapshots/xtask-source-bundle-help.txt"),
    );
}

fn assert_help_snapshot(args: &[&str], expected: &str) {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .output()
        .expect("xtask help command should run");
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout).expect("UTF-8 help"),
        expected
    );
}
