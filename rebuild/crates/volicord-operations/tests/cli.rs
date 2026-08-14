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
fn cli_runs_derived_repair_and_reindex_and_rejects_canonical_repair(
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
                "analyze",
                &project,
            ],
            &mut output,
            &mut error
        ),
        CliExit::SUCCESS,
        "{}",
        String::from_utf8_lossy(&error)
    );

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
        CliExit::FAILURE
    );
    assert!(String::from_utf8_lossy(&error).contains("unsupported repair scope"));

    output.clear();
    error.clear();
    assert_eq!(
        run_cli(
            [
                "--runtime",
                runtime.to_str().ok_or("runtime path")?,
                "repair",
                &project,
                "derived-analysis"
            ],
            &mut output,
            &mut error
        ),
        CliExit::SUCCESS,
        "{}",
        String::from_utf8_lossy(&error)
    );
    let repair: Value = serde_json::from_slice(&output)?;
    assert_eq!(repair["kind"], "derivedanalysisrepair");
    assert!(repair["analysis_snapshot"].is_string());

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
    assert!(reindex["analysis_snapshot"].is_string());
    Ok(())
}

#[test]
fn cli_guarded_fallback_preserves_request_revision_fingerprint_and_source_linkage(
) -> Result<(), Box<dyn std::error::Error>> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let temporary = tempfile::tempdir()?;
    let runtime = temporary.path().join("runtime");
    let repository = temporary.path().join("repository");
    fs::create_dir(&repository)?;
    let runtime_text = runtime.to_str().ok_or("runtime path")?;
    let repository_text = repository.to_str().ok_or("repository path")?;
    let mut output = Vec::new();
    let mut error = Vec::new();
    assert_eq!(
        run_cli(
            [
                "--runtime",
                runtime_text,
                "project",
                "init",
                "Fixture",
                "--repository",
                repository_text,
            ],
            &mut output,
            &mut error,
        ),
        CliExit::SUCCESS
    );
    let initialized: Value = serde_json::from_slice(&output)?;
    let project = initialized["project_id"]
        .as_str()
        .ok_or("missing project")?
        .to_owned();
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros();
    let expiration = (now + 60_000_000).to_string();

    output.clear();
    error.clear();
    assert_eq!(
        run_cli(
            [
                "--runtime",
                runtime_text,
                "guarded",
                "request",
                &project,
                "external-publication",
                "publish",
                "registry.example/release",
                "publish release",
                "external users can observe it",
                &expiration,
                "artifact:release",
            ],
            &mut output,
            &mut error,
        ),
        CliExit::SUCCESS,
        "{}",
        String::from_utf8_lossy(&error)
    );
    let request: Value = serde_json::from_slice(&output)?;
    let identity = request["confirmation_request_identity"]
        .as_str()
        .ok_or("missing request ID")?
        .to_owned();
    let revision = request["request_revision"]
        .as_u64()
        .ok_or("missing revision")?
        .to_string();
    let fingerprint = request["effect_fingerprint"]
        .as_str()
        .ok_or("missing fingerprint")?
        .to_owned();

    output.clear();
    error.clear();
    assert_eq!(
        run_cli(
            [
                "--runtime",
                runtime_text,
                "guarded",
                "confirm",
                &identity,
                &revision,
                &fingerprint,
                "codex",
                "session-42",
                "I confirm this exact effect",
            ],
            &mut output,
            &mut error,
        ),
        CliExit::SUCCESS,
        "{}",
        String::from_utf8_lossy(&error)
    );
    let confirmation: Value = serde_json::from_slice(&output)?;
    assert_eq!(confirmation["confirmation_request_identity"], identity);
    assert_eq!(
        confirmation["request_revision"],
        request["request_revision"]
    );
    assert_eq!(confirmation["effect_fingerprint"], fingerprint);
    assert!(confirmation["user_response_source_id"].as_str().is_some());
    Ok(())
}
