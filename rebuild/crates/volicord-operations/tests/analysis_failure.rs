use std::{error::Error, fs};
use volicord_operations::{LocalOperations, RuntimeLayout};

#[test]
fn primary_analysis_failure_survives_receipt_storage_failure() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let repository = temp.path().join("repo");
    fs::create_dir(&repository)?;
    fs::write(repository.join("main.py"), "def value():\n    return 7\n")?;
    let ops = LocalOperations::new(RuntimeLayout::new(temp.path().join("runtime"))?);
    let project = ops
        .initialize_project("Controlled analysis failure", Some(&repository))?
        .project
        .id;
    let before = ops.canonical_basis(project)?;
    fs::rename(repository, temp.path().join("offline"))?;
    let attempts = ops.layout().artifacts_dir().join("analysis-attempts");
    fs::create_dir_all(&attempts)?;
    fs::write(
        attempts.join(project.to_string()),
        b"controlled receipt storage blocker",
    )?;
    let failure = ops
        .analyze(project, vec![])
        .expect_err("both analysis and recording fail");
    assert_eq!(failure.message(), "bound repository is unavailable");
    assert!(
        failure.source().is_some(),
        "the primary source chain survives"
    );
    let diagnostic = failure
        .analysis_receipt_failure()
        .ok_or("secondary failure missing")?;
    assert!(diagnostic.primary_failed);
    assert_eq!(diagnostic.project_id, project);
    assert_eq!(diagnostic.published_analysis_snapshot, None);
    assert_eq!(
        diagnostic.recording_error().message(),
        "cannot prepare analysis attempt receipts"
    );
    assert!(diagnostic.recording_error().source().is_some());
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = volicord_operations::run_cli_with_input(
        [
            "--runtime".to_owned(),
            ops.layout().root().to_string_lossy().into_owned(),
            "--project".into(),
            project.to_string(),
            "analyze".into(),
        ],
        &mut std::io::empty(),
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(code.code(), 1);
    assert!(stdout.is_empty());
    let stderr = String::from_utf8(stderr)?;
    assert!(stderr.contains("bound repository is unavailable"));
    assert!(stderr.contains("analysis_attempt_receipt_unavailable"));
    assert!(stderr.contains("\"primary_failed\":true"));
    assert!(!stderr.contains("controlled receipt storage blocker"));
    assert_eq!(before, ops.canonical_basis(project)?);
    Ok(())
}

#[test]
fn published_analysis_is_distinct_from_failed_receipt_and_survives_retry(
) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let repository = temp.path().join("repo");
    fs::create_dir(&repository)?;
    fs::write(repository.join("main.py"), "def value():\n    return 7\n")?;
    let ops = LocalOperations::new(RuntimeLayout::new(temp.path().join("runtime"))?);
    let project = ops
        .initialize_project("Controlled publication receipt", Some(&repository))?
        .project
        .id;
    let attempts = ops.layout().artifacts_dir().join("analysis-attempts");
    fs::create_dir_all(&attempts)?;
    let blocker = attempts.join(project.to_string());
    fs::write(&blocker, b"private storage blocker")?;
    let error = ops
        .analyze(project, vec![])
        .expect_err("published analysis has no receipt");
    let diagnostic = error.analysis_receipt_failure().ok_or("diagnostic")?;
    assert!(!diagnostic.primary_failed);
    assert!(error.message().contains("analysis was published"));
    let snapshot = diagnostic
        .published_analysis_snapshot
        .ok_or("published snapshot identity")?;
    assert_eq!(
        diagnostic.published_operation_state,
        Some(volicord_operations::OperationState::Partial)
    );
    let path = ops
        .layout()
        .analysis_project_dir(project)
        .join(format!("{snapshot}.json"));
    let original = fs::read(&path)?;
    assert!(!original.is_empty());
    fs::remove_file(&blocker)?;
    let later = ops.analyze(project, vec![])?;
    assert_eq!(later.state, volicord_operations::OperationState::Partial);
    assert_eq!(
        fs::read(&path)?,
        original,
        "later success cannot erase an earlier publication"
    );
    Ok(())
}
