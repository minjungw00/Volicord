use rusqlite::Connection;
use serde_json::Value;
use std::{
    collections::BTreeSet,
    fs,
    sync::{Arc, Barrier},
    thread,
};
use volicord_operations::{run_cli, CliExit};

#[test]
fn document_cli_reports_requested_language_realizer_unavailable_without_publishing_english(
) -> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let runtime = temporary.path().join("runtime");
    let runtime_text = runtime.to_str().ok_or("runtime path")?;
    let mut output = Vec::new();
    let mut error = Vec::new();
    assert_eq!(
        run_cli(
            ["--runtime", runtime_text, "project", "init", "Language CLI"],
            &mut output,
            &mut error,
        ),
        CliExit::SUCCESS
    );
    let initialized: Value = serde_json::from_slice(&output)?;
    let project = initialized["project_id"]
        .as_str()
        .ok_or("missing Project ID")?;

    output.clear();
    error.clear();
    assert_eq!(
        run_cli(
            [
                "--runtime",
                runtime_text,
                "documents",
                "preview",
                project,
                "handoff-resume",
                "markdown",
                "es",
            ],
            &mut output,
            &mut error,
        ),
        CliExit::SUCCESS,
        "{}",
        String::from_utf8_lossy(&error)
    );
    let preview: Value = serde_json::from_slice(&output)?;
    assert_eq!(preview["outcome"], "unavailable");
    assert_eq!(preview["requested_language"], "es");
    assert!(preview.get("content").is_none());
    assert_eq!(preview["published"], false);
    Ok(())
}

#[test]
fn candidate_cli_rejects_dependency_failures_as_empty_success(
) -> Result<(), Box<dyn std::error::Error>> {
    for fault in ["unsupported", "corrupt", "unavailable"] {
        let temporary = tempfile::tempdir()?;
        let runtime = temporary.path().join("runtime");
        let runtime_text = runtime.to_str().ok_or("runtime path")?;
        let mut output = Vec::new();
        let mut error = Vec::new();
        assert_eq!(
            run_cli(
                [
                    "--runtime",
                    runtime_text,
                    "project",
                    "init",
                    "Candidate CLI"
                ],
                &mut output,
                &mut error,
            ),
            CliExit::SUCCESS,
            "{fault}: {}",
            String::from_utf8_lossy(&error)
        );
        let initialized: Value = serde_json::from_slice(&output)?;
        let project = initialized["project_id"]
            .as_str()
            .ok_or("missing Project ID")?;
        let candidate_store = runtime.join("candidates.sqlite3");
        match fault {
            "unsupported" => {
                Connection::open(&candidate_store)?.execute(
                    "UPDATE metadata SET value = '999' WHERE key = 'schema_version'",
                    [],
                )?;
            }
            "corrupt" => {
                Connection::open(&candidate_store)?.execute("DROP TABLE candidates", [])?;
            }
            "unavailable" => {
                fs::remove_file(&candidate_store)?;
                fs::create_dir(&candidate_store)?;
            }
            _ => unreachable!(),
        }
        output.clear();
        error.clear();
        assert_eq!(
            run_cli(
                ["--runtime", runtime_text, "candidates", project],
                &mut output,
                &mut error,
            ),
            CliExit::SUCCESS,
            "{fault}: {}",
            String::from_utf8_lossy(&error)
        );
        let inspection: Value = serde_json::from_slice(&output)?;
        assert_eq!(inspection["health"], "degraded", "{fault}: {inspection}");
        assert_eq!(inspection["candidates"], serde_json::json!([]));
    }
    Ok(())
}

#[test]
fn concurrent_cli_writers_preserve_every_committed_source() -> Result<(), Box<dyn std::error::Error>>
{
    const WRITERS: usize = 8;
    let temporary = tempfile::tempdir()?;
    let runtime = temporary.path().join("runtime");
    let runtime_text = runtime.to_str().ok_or("runtime path")?;
    let mut output = Vec::new();
    let mut error = Vec::new();
    assert_eq!(
        run_cli(
            [
                "--runtime",
                runtime_text,
                "project",
                "init",
                "Concurrent CLI",
            ],
            &mut output,
            &mut error,
        ),
        CliExit::SUCCESS,
        "{}",
        String::from_utf8_lossy(&error)
    );
    let initialized: Value = serde_json::from_slice(&output)?;
    let project = initialized["project_id"]
        .as_str()
        .ok_or("missing Project ID")?
        .to_owned();
    let barrier = Arc::new(Barrier::new(WRITERS));
    let mut writers = Vec::new();
    for index in 0..WRITERS {
        let runtime = runtime.clone();
        let project = project.clone();
        let barrier = Arc::clone(&barrier);
        writers.push(thread::spawn(move || {
            barrier.wait();
            let mut output = Vec::new();
            let mut error = Vec::new();
            let runtime = runtime.to_str().expect("runtime path");
            let turn = format!("concurrent CLI writer {index}");
            assert_eq!(
                run_cli(
                    [
                        "--runtime",
                        runtime,
                        "canonical",
                        "user-source",
                        &project,
                        "cli-test",
                        "bounded-concurrency",
                        &turn,
                    ],
                    &mut output,
                    &mut error,
                ),
                CliExit::SUCCESS,
                "writer {index}: {}",
                String::from_utf8_lossy(&error)
            );
            let result: Value = serde_json::from_slice(&output).expect("writer JSON");
            result["identity"]
                .as_str()
                .expect("Source identity")
                .to_owned()
        }));
    }
    let identities = writers
        .into_iter()
        .map(|writer| writer.join().expect("CLI writer"))
        .collect::<BTreeSet<_>>();
    assert_eq!(identities.len(), WRITERS);

    output.clear();
    error.clear();
    assert_eq!(
        run_cli(
            ["--runtime", runtime_text, "canonical", "inspect", &project],
            &mut output,
            &mut error,
        ),
        CliExit::SUCCESS,
        "{}",
        String::from_utf8_lossy(&error)
    );
    let inspection: Value = serde_json::from_slice(&output)?;
    let persisted = inspection["records"]
        .as_array()
        .ok_or("canonical records")?
        .iter()
        .filter_map(|record| record["identity"].as_str())
        .collect::<BTreeSet<_>>();
    assert!(identities
        .iter()
        .all(|identity| persisted.contains(identity.as_str())));
    Ok(())
}

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

#[test]
fn cli_handoff_requires_and_records_an_explicit_target_without_changing_other_kinds(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_context::{CanonicalReadOptions, ProjectId, Store};

    let temporary = tempfile::tempdir()?;
    let runtime = temporary.path().join("runtime");
    let runtime_text = runtime.to_str().ok_or("runtime path")?;
    let mut output = Vec::new();
    let mut error = Vec::new();
    assert_eq!(
        run_cli(
            [
                "--runtime",
                runtime_text,
                "project",
                "init",
                "Checkpoint Fixture",
            ],
            &mut output,
            &mut error,
        ),
        CliExit::SUCCESS
    );
    let initialized: Value = serde_json::from_slice(&output)?;
    let project = initialized["project_id"]
        .as_str()
        .ok_or("missing Project ID")?
        .to_owned();

    output.clear();
    error.clear();
    assert_eq!(
        run_cli(
            [
                "--runtime",
                runtime_text,
                "canonical",
                "user-source",
                &project,
                "cli",
                "checkpoint-test",
                "record an explicit checkpoint",
            ],
            &mut output,
            &mut error,
        ),
        CliExit::SUCCESS
    );
    let source: Value = serde_json::from_slice(&output)?;
    let source = source["identity"]
        .as_str()
        .ok_or("missing Source ID")?
        .to_owned();

    output.clear();
    error.clear();
    assert_eq!(
        run_cli(
            [
                "--runtime",
                runtime_text,
                "checkpoint",
                "record",
                &project,
                "handoff",
                &source,
                "handoff goal",
                "continue work",
            ],
            &mut output,
            &mut error,
        ),
        CliExit::USAGE
    );
    assert!(String::from_utf8_lossy(&error).contains("missing explicit handoff target"));

    let project_id = ProjectId::from_bytes(parse_hex_identity(&project)?);
    let store = Store::open(runtime.join("canonical.sqlite3"))?;
    assert!(store
        .read_canonical_basis(
            project_id,
            CanonicalReadOptions {
                include_checkpoint_history: true,
            },
        )?
        .checkpoint_history
        .is_empty());
    drop(store);

    for kind in ["completion", "pause"] {
        output.clear();
        error.clear();
        assert_eq!(
            run_cli(
                [
                    "--runtime",
                    runtime_text,
                    "checkpoint",
                    "record",
                    &project,
                    kind,
                    &source,
                    "ordinary checkpoint goal",
                    "continue ordinary work",
                ],
                &mut output,
                &mut error,
            ),
            CliExit::SUCCESS,
            "{}",
            String::from_utf8_lossy(&error)
        );
    }
    let store = Store::open(runtime.join("canonical.sqlite3"))?;
    let before_handoff = store.read_canonical_basis(
        project_id,
        CanonicalReadOptions {
            include_checkpoint_history: true,
        },
    )?;
    assert_eq!(before_handoff.checkpoint_history.len(), 2);
    assert!(before_handoff
        .checkpoint_history
        .iter()
        .all(|checkpoint| checkpoint.handoff_to.is_none()));
    drop(store);

    output.clear();
    error.clear();
    assert_eq!(
        run_cli(
            [
                "--runtime",
                runtime_text,
                "checkpoint",
                "record",
                &project,
                "handoff",
                &source,
                "handoff goal",
                "continue work",
                "next Codex session",
            ],
            &mut output,
            &mut error,
        ),
        CliExit::SUCCESS,
        "{}",
        String::from_utf8_lossy(&error)
    );
    let store = Store::open(runtime.join("canonical.sqlite3"))?;
    let after_handoff = store.read_canonical_basis(
        project_id,
        CanonicalReadOptions {
            include_checkpoint_history: true,
        },
    )?;
    assert_eq!(after_handoff.checkpoint_history.len(), 3);
    assert_eq!(
        after_handoff
            .latest_checkpoint
            .as_ref()
            .and_then(|checkpoint| checkpoint.handoff_to.as_deref()),
        Some("next Codex session")
    );
    Ok(())
}

fn parse_hex_identity(value: &str) -> Result<[u8; 16], Box<dyn std::error::Error>> {
    if value.len() != 32 {
        return Err("identity must contain 32 hexadecimal digits".into());
    }
    let mut bytes = [0_u8; 16];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] = u8::from_str_radix(std::str::from_utf8(pair)?, 16)?;
    }
    Ok(bytes)
}
