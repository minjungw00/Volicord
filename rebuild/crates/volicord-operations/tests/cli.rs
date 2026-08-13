use serde_json::Value;
use std::fs;
use volicord_operations::{run_cli, CliExit};

#[test]
fn cli_initializes_binds_analyzes_and_inspects_without_raw_storage_access(
) -> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let runtime = temporary.path().join("runtime");
    let repository = temporary.path().join("repository");
    fs::create_dir_all(repository.join("src"))?;
    fs::write(
        repository.join("src/main.py"),
        "def main():\n    return 0\n",
    )?;

    let mut output = Vec::new();
    let mut error = Vec::new();
    let exit = run_cli(
        [
            "--runtime",
            runtime.to_str().ok_or("runtime path is not UTF-8")?,
            "project",
            "init",
            "CLI Fixture",
            "--repository",
            repository.to_str().ok_or("repository path is not UTF-8")?,
        ],
        &mut output,
        &mut error,
    );
    assert_eq!(
        exit,
        CliExit::SUCCESS,
        "{}",
        String::from_utf8_lossy(&error)
    );
    let initialized: Value = serde_json::from_slice(&output)?;
    let project = initialized["project_id"]
        .as_str()
        .ok_or("missing Project ID")?;

    output.clear();
    error.clear();
    let exit = run_cli(
        [
            "--runtime",
            runtime.to_str().ok_or("runtime path is not UTF-8")?,
            "analyze",
            project,
        ],
        &mut output,
        &mut error,
    );
    assert_eq!(
        exit,
        CliExit::SUCCESS,
        "{}",
        String::from_utf8_lossy(&error)
    );
    let analyzed: Value = serde_json::from_slice(&output)?;
    assert!(matches!(
        analyzed["state"].as_str(),
        Some("succeeded" | "partial")
    ));

    output.clear();
    error.clear();
    let exit = run_cli(
        [
            "--runtime",
            runtime.to_str().ok_or("runtime path is not UTF-8")?,
            "canonical",
            "inspect",
            project,
        ],
        &mut output,
        &mut error,
    );
    assert_eq!(
        exit,
        CliExit::SUCCESS,
        "{}",
        String::from_utf8_lossy(&error)
    );
    let inspection: Value = serde_json::from_slice(&output)?;
    assert_eq!(inspection["operation"], "canonical_inspect");
    assert!(inspection["records"]
        .as_array()
        .is_some_and(|records| !records.is_empty()));
    Ok(())
}

#[test]
fn cli_reports_distinct_unsupported_repair_and_reindex_results(
) -> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let runtime = temporary.path().join("runtime");
    let repository = temporary.path().join("repository");
    fs::create_dir(&repository)?;
    let mut output = Vec::new();
    let mut error = Vec::new();
    assert_eq!(
        run_cli(
            [
                "--runtime",
                runtime.to_str().ok_or("runtime path")?,
                "project",
                "init",
                "Fixture",
                "--repository",
                repository.to_str().ok_or("repository path")?
            ],
            &mut output,
            &mut error
        ),
        CliExit::SUCCESS
    );
    let initialized: Value = serde_json::from_slice(&output)?;
    let project = initialized["project_id"]
        .as_str()
        .ok_or("missing project")?
        .to_owned();

    output.clear();
    error.clear();
    assert_eq!(
        run_cli(
            [
                "--runtime",
                runtime.to_str().ok_or("runtime path")?,
                "repair",
                &project,
                "canonical"
            ],
            &mut output,
            &mut error
        ),
        CliExit::SUCCESS
    );
    let repair: Value = serde_json::from_slice(&output)?;
    assert_eq!(repair["kind"], "authoritativerepair");

    output.clear();
    error.clear();
    assert_eq!(
        run_cli(
            [
                "--runtime",
                runtime.to_str().ok_or("runtime path")?,
                "reindex",
                &project
            ],
            &mut output,
            &mut error
        ),
        CliExit::SUCCESS
    );
    let reindex: Value = serde_json::from_slice(&output)?;
    assert_eq!(reindex["kind"], "derivedrebuild");
    Ok(())
}
