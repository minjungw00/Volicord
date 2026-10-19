#[path = "support/scoped_check_fixture.rs"]
#[allow(dead_code)]
mod fixture;
use std::{error::Error, fs};
use volicord_context::*;
#[test]
fn check_target_fingerprints_support_non_ascii_empty_and_polyglot_files_without_claiming_execution_coverage(
) -> Result<(), Box<dyn Error>> {
    let f = fixture::fixture()?;
    let root = f.temp.path().join("repo");
    let files = [
        ("notes.custom", "한글🙂\n"),
        ("empty.py", ""),
        ("module.ts", "// 日本語\nexport const n = 7;\n"),
        ("lib.rs", "// Ελληνικά\npub fn n() -> i32 { 7 }\n"),
        ("main.c", "/* العربية */\nint main(void) { return 0; }\n"),
    ];
    for (path, text) in files {
        fs::write(root.join(path), text)?;
    }
    let mut declaration = fixture::declaration();
    declaration.target_paths = files.iter().map(|(p, _)| p.to_string()).collect();
    declaration.coverage = None;
    let source = f
        .ops
        .run_work_check(fixture::command(&f.ops, f.project, f.work)?, declaration)?
        .source_id;
    let mut request = fixture::recording(f.work, source, true);
    request.assertion_state = AssertionState::NotObserved;
    request.assertion_evidence = None;
    request.outcome = "Artifact inventory sampled; no language execution coverage claimed".into();
    f.ops.record_work_check(f.project, request)?;
    let cp = f
        .ops
        .canonical_basis(f.project)?
        .latest_checkpoint
        .ok_or("checkpoint")?;
    assert_eq!(cp.verification[0].state, VerificationState::Partial);
    let run = cp.verification[0].scoped_run.as_ref().ok_or("run")?;
    assert!(run.target_unchanged_during_run);
    assert_eq!(run.artifacts.len(), 5);
    use sha2::{Digest, Sha256};
    for (artifact, (path, text)) in run.artifacts.iter().zip(files) {
        assert_eq!(artifact.path, path);
        assert_eq!(
            artifact.sha256,
            format!("{:x}", Sha256::digest(text.as_bytes()))
        );
    }
    Ok(())
}
