use std::{path::Path, process::Command};

// Disposable repository evidence only. Never use the contributor's Git identity,
// hooks, signing configuration, or the repository containing these tests.
pub fn git(repository: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=Work fixture",
            "-c",
            "user.email=work-fixture@example.invalid",
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(arguments)
        .current_dir(repository)
        .output()
        .expect("execute fixture Git");
    assert!(
        output.status.success(),
        "git {arguments:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("Git fixture output is UTF-8")
}
